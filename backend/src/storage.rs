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
/// Browser-compatible SigV4 PUT: Content-Length and Content-Type are both signed.
/// The browser sets Content-Length from the file body; completion revalidates bytes.
pub fn presigned_put(
    config: &Config,
    key: &str,
    content_type: &str,
    size: u64,
) -> Result<serde_json::Value> {
    presigned_put_credentials(
        &UploadSigningConfig {
            endpoint: &config.storage_endpoint,
            bucket: &config.storage_bucket,
            region: &config.storage_region,
            access_key: &config.storage_access_key,
            secret_key: config.storage_secret_key.as_str(),
        },
        key,
        content_type,
        size,
    )
}

pub struct UploadSigningConfig<'a> {
    pub endpoint: &'a str,
    pub bucket: &'a str,
    pub region: &'a str,
    pub access_key: &'a str,
    pub secret_key: &'a str,
}

pub fn presigned_put_credentials(
    config: &UploadSigningConfig<'_>,
    key: &str,
    content_type: &str,
    size: u64,
) -> Result<serde_json::Value> {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    fn sign(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut m =
            <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
        m.update(data);
        m.finalize().into_bytes().to_vec()
    }
    if size == 0
        || size > 5 * 1024 * 1024
        || !matches!(content_type, "image/png" | "image/jpeg" | "image/webp")
    {
        return Err(ApiError::invalid("Invalid evidence upload metadata."));
    }
    let now = chrono::Utc::now();
    let date = now.format("%Y%m%d").to_string();
    let stamp = now.format("%Y%m%dT%H%M%SZ").to_string();
    let scope = format!("{}/{}/s3/aws4_request", date, config.region);
    let credential = format!("{}/{}", config.access_key, scope);
    let mut url = url::Url::parse(config.endpoint).map_err(|_| ApiError::internal())?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| ApiError::internal())?;
        segments.pop_if_empty().push(config.bucket);
        for segment in key.split('/') {
            segments.push(segment);
        }
    }
    let host = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().ok_or_else(ApiError::internal)?),
        None => url.host_str().ok_or_else(ApiError::internal)?.to_owned(),
    };
    let signed_headers = "content-length;content-type;host";
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("X-Amz-Algorithm", "AWS4-HMAC-SHA256")
        .append_pair("X-Amz-Credential", &credential)
        .append_pair("X-Amz-Date", &stamp)
        .append_pair("X-Amz-Expires", "300")
        .append_pair("X-Amz-SignedHeaders", signed_headers)
        .finish()
        .replace('+', "%20");
    let headers = format!("content-length:{size}\ncontent-type:{content_type}\nhost:{host}\n");
    let canonical = format!(
        "PUT\n{}\n{query}\n{headers}\n{signed_headers}\nUNSIGNED-PAYLOAD",
        url.path()
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{stamp}\n{scope}\n{}",
        hex::encode(Sha256::digest(canonical.as_bytes()))
    );
    let day = sign(
        format!("AWS4{}", config.secret_key).as_bytes(),
        date.as_bytes(),
    );
    let region = sign(&day, config.region.as_bytes());
    let service = sign(&region, b"s3");
    let signing = sign(&service, b"aws4_request");
    url.set_query(Some(&format!(
        "{query}&X-Amz-Signature={}",
        hex::encode(sign(&signing, string_to_sign.as_bytes()))
    )));
    Ok(
        serde_json::json!({"url":url.to_string(),"method":"PUT","headers":{"Content-Type":content_type},"size_bytes":size,"expires_in":300,"instructions":"Send the original file as the PUT body. Content type and exact byte length are signed; completion validates image bytes and preserves an immutable copy."}),
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
