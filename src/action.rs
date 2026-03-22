use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    components::{profiles::ProfileInfo, s3_buckets::BucketInfo},
    resource_selector::ResourceType,
};

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    // Command bar
    OpenCommandBar,
    CloseBar,
    SubmitCommand(String),
    // Navigation
    SwitchView(ResourceType),
    // Profiles
    LoadProfiles,
    #[serde(skip)]
    ProfilesLoaded(Vec<ProfileInfo>),
    ProfilesLoadError(String),
    SelectNext,
    SelectPrevious,
    Confirm,
    #[serde(skip)]
    ProfileSelected {
        name: String,
        region: Option<String>,
    },
    // S3
    LoadS3Buckets,
    #[serde(skip)]
    S3BucketsLoaded(Vec<BucketInfo>),
    S3BucketsError(String),
}
