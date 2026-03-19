use aws_sdk_s3::Client as S3Client;

pub struct AwsState {
    pub sdk_config: aws_config::SdkConfig,
    pub s3_client: S3Client,
}

impl AwsState {
    pub async fn init() -> color_eyre::Result<Self> {
        let sdk_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let s3_client = S3Client::new(&sdk_config);

        Ok(Self {
            sdk_config,
            s3_client,
        })
    }

    pub fn region(&self) -> Option<&str> {
        self.sdk_config.region().map(|r| r.as_ref())
    }
}
