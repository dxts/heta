use std::sync::Arc;

use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use tokio::sync::RwLock;

pub struct AwsState {
    pub sdk_config: aws_config::SdkConfig,
    /// Shared S3 client — wrapped in Arc<RwLock> so components can hold
    /// a handle that always points to the current client, even after
    /// profile switches.
    pub s3_client: Arc<RwLock<S3Client>>,
    pub profile: String,
}

impl AwsState {
    pub async fn init() -> color_eyre::Result<Self> {
        let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let s3_client = Arc::new(RwLock::new(S3Client::new(&sdk_config)));

        Ok(Self {
            sdk_config,
            s3_client,
            profile: "default".into(),
        })
    }

    pub async fn reload_for_profile(
        &mut self,
        profile: &str,
        region: Option<&str>,
    ) -> color_eyre::Result<()> {
        let mut loader = aws_config::defaults(BehaviorVersion::latest()).profile_name(profile);

        if let Some(r) = region {
            loader = loader.region(aws_config::Region::new(r.to_string()));
        }

        self.sdk_config = loader.load().await;
        // Swap the client behind the lock — all components see the new one immediately
        *self.s3_client.write().await = S3Client::new(&self.sdk_config);
        self.profile = profile.to_string();
        Ok(())
    }

    pub fn region(&self) -> Option<&str> {
        self.sdk_config.region().map(|r| r.as_ref())
    }
}