// Live storage compatibility check; no blockchain signer or factory setting is required.
use anyhow::{Context, ensure};
use oppor_backend::storage::{UploadSigningConfig, presigned_put_credentials};
use zeroize::Zeroizing;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let endpoint = std::env::var("OBJECT_STORAGE_ENDPOINT")?;
    let bucket = std::env::var("OBJECT_STORAGE_BUCKET")?;
    let region = std::env::var("OBJECT_STORAGE_REGION")?;
    let access = std::env::var("OBJECT_STORAGE_ACCESS_KEY")?;
    let secret = Zeroizing::new(std::env::var("OBJECT_STORAGE_SECRET_KEY")?);
    let config = UploadSigningConfig {
        endpoint: &endpoint,
        bucket: &bucket,
        region: &region,
        access_key: &access,
        secret_key: &secret,
    };
    let key = format!("staging/ops-smoke/{}", uuid::Uuid::new_v4());
    let bytes = hex::decode("89504e470d0a1a0a00000000")?;
    let signed = presigned_put_credentials(&config, &key, "image/png", bytes.len() as u64)
        .context("Sign upload")?;
    let url = signed["url"].as_str().context("Missing URL")?;
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let response = http
        .put(url)
        .header("Content-Type", "image/png")
        .body(bytes.clone())
        .send()
        .await?;
    ensure!(
        response.status().is_success(),
        "R2 signed PUT rejected with status {}",
        response.status()
    );
    let bad = http
        .put(url)
        .header("Content-Type", "image/jpeg")
        .body(bytes.clone())
        .send()
        .await?;
    ensure!(
        !bad.status().is_success(),
        "R2 accepted a changed signed content type"
    );
    let bad = http
        .put(url)
        .header("Content-Type", "image/png")
        .body(vec![0u8; bytes.len() + 1])
        .send()
        .await?;
    ensure!(
        !bad.status().is_success(),
        "R2 accepted a changed signed byte length"
    );
    let s3 = object_store::aws::AmazonS3Builder::new()
        .with_endpoint(&endpoint)
        .with_bucket_name(&bucket)
        .with_region(&region)
        .with_access_key_id(&access)
        .with_secret_access_key(secret.as_str())
        .build()?;
    use object_store::ObjectStoreExt;
    let path = object_store::path::Path::from(key);
    let downloaded = s3.get(&path).await?.bytes().await?;
    ensure!(
        downloaded.as_ref() == bytes.as_slice(),
        "R2 returned different evidence bytes"
    );
    s3.delete(&path).await?;
    println!("R2 PUT, signed metadata rejection, readback, and cleanup passed.");
    Ok(())
}
