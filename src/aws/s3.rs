use aws_sdk_s3::Client as S3Client;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketInfo {
    pub name: String,
    pub region: Option<String>,
    pub creation_date: Option<String>,
}

/// Fetches all S3 buckets visible to the current credentials.
pub async fn list_buckets(client: &S3Client) -> color_eyre::Result<Vec<BucketInfo>> {
    let resp = client.list_buckets().send().await?;

    let buckets = resp
        .buckets()
        .iter()
        .map(|b| BucketInfo {
            name: b.name().unwrap_or("—").to_string(),
            region: None, // bucket-level region requires a per-bucket HEAD call
            creation_date: b.creation_date().map(|d| {
                d.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime)
                    .unwrap_or_default()
            }),
        })
        .collect();

    Ok(buckets)
}
