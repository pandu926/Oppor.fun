use crate::{
    config::Config,
    error::{ApiError, Result},
};
use bytes::Bytes;
use object_store::{
    ObjectStore, ObjectStoreExt, PutMode, PutOptions,
    aws::{AmazonS3, AmazonS3Builder},
    path::Path,
    signer::Signer,
};
use std::time::Duration;

#[derive(Clone)]
pub struct Storage {
    pub client: std::sync::Arc<AmazonS3>,
}
impl Storage {
    pub fn new(config: &Config) -> Result<Self> {
        let client = AmazonS3Builder::new()
            .with_bucket_name(&config.storage_bucket)
            .with_region(&config.storage_region)
            .with_endpoint(&config.storage_endpoint)
            .with_access_key_id(&config.storage_access_key)
            .with_secret_access_key(config.storage_secret_key.as_str())
            .with_allow_http(!config.production)
            .with_virtual_hosted_style_request(false)
            .build()
            .map_err(|e| {
                tracing::error!(error=%e,"Object storage configuration failed");
                ApiError::internal()
            })?;
        Ok(Self {
            client: std::sync::Arc::new(client),
        })
    }
    pub async fn signed_put(&self, key: &str) -> Result<String> {
        self.client
            .signed_url(
                reqwest::Method::PUT,
                &Path::from(key),
                Duration::from_secs(300),
            )
            .await
            .map(|u| u.to_string())
            .map_err(|_| ApiError::unavailable())
    }
    pub async fn signed_get(&self, key: &str) -> Result<String> {
        self.client
            .signed_url(
                reqwest::Method::GET,
                &Path::from(key),
                Duration::from_secs(120),
            )
            .await
            .map(|u| u.to_string())
            .map_err(|_| ApiError::unavailable())
    }
    pub async fn read_evidence(
        &self,
        key: &str,
        expected: u64,
        content_type: &str,
    ) -> Result<Bytes> {
        let result = self
            .client
            .get(&Path::from(key))
            .await
            .map_err(|_| ApiError::unavailable())?;
        if result.meta.size != expected || expected > 5 * 1024 * 1024 {
            return Err(ApiError::invalid(
                "The uploaded file size does not match the declared size.",
            ));
        }
        let bytes = result.bytes().await.map_err(|_| ApiError::unavailable())?;
        if !valid_magic(&bytes, content_type) {
            return Err(ApiError::invalid(
                "The uploaded file content does not match an allowed image format.",
            ));
        }
        Ok(bytes)
    }
    pub async fn put_immutable(&self, key: &str, bytes: Bytes) -> Result<()> {
        let path = Path::from(key);
        match self
            .client
            .put_opts(
                &path,
                bytes.clone().into(),
                PutOptions {
                    mode: PutMode::Create,
                    ..Default::default()
                },
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(object_store::Error::AlreadyExists { .. }) => {
                let result = self
                    .client
                    .get(&path)
                    .await
                    .map_err(|_| ApiError::unavailable())?;
                if result.meta.size != bytes.len() as u64 {
                    return Err(ApiError::internal());
                }
                let existing = result.bytes().await.map_err(|_| ApiError::unavailable())?;
                if existing != bytes {
                    return Err(ApiError::internal());
                }
                Ok(())
            }
            Err(_) => Err(ApiError::unavailable()),
        }
    }
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete(&Path::from(key))
            .await
            .map_err(|_| ApiError::unavailable())
    }
}
pub fn presigned_post(
    config: &Config,
    key: &str,
    content_type: &str,
    size: u64,
) -> Result<serde_json::Value> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    fn sign(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut m =
            <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
        m.update(data);
        m.finalize().into_bytes().to_vec()
    }
    let now = chrono::Utc::now();
    let date = now.format("%Y%m%d").to_string();
    let stamp = now.format("%Y%m%dT%H%M%SZ").to_string();
    let credential = format!(
        "{}/{}/{}/s3/aws4_request",
        config.storage_access_key, date, config.storage_region
    );
    let policy = serde_json::json!({"expiration":(now+chrono::Duration::minutes(5)).to_rfc3339(),"conditions":[{"bucket":config.storage_bucket},{"key":key},{"Content-Type":content_type},["content-length-range",size,size],{"x-amz-algorithm":"AWS4-HMAC-SHA256"},{"x-amz-credential":credential},{"x-amz-date":stamp}]});
    let encoded = STANDARD.encode(serde_json::to_vec(&policy).map_err(|_| ApiError::internal())?);
    let day = sign(
        format!("AWS4{}", config.storage_secret_key.as_str()).as_bytes(),
        date.as_bytes(),
    );
    let region = sign(&day, config.storage_region.as_bytes());
    let service = sign(&region, b"s3");
    let signing = sign(&service, b"aws4_request");
    let mut url = url::Url::parse(&config.storage_endpoint).map_err(|_| ApiError::internal())?;
    url.path_segments_mut()
        .map_err(|_| ApiError::internal())?
        .pop_if_empty()
        .push(&config.storage_bucket);
    Ok(
        serde_json::json!({"url":url.to_string(),"method":"POST","fields":{"key":key,"Content-Type":content_type,"policy":encoded,"x-amz-algorithm":"AWS4-HMAC-SHA256","x-amz-credential":credential,"x-amz-date":stamp,"x-amz-signature":hex::encode(sign(&signing,encoded.as_bytes()))},"expires_in":300,"instructions":"Send multipart form fields followed by the file field. The signed policy enforces the exact file size and key."}),
    )
}
pub fn valid_magic(b: &[u8], t: &str) -> bool {
    match t {
        "image/png" => b.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => b.len() > 3 && b.starts_with(&[0xff, 0xd8, 0xff]),
        "image/webp" => b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP",
        _ => false,
    }
}
