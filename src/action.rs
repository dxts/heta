use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    components::{profiles::ProfileInfo, s3_buckets::BucketInfo, s3_objects::ObjectInfo},
    page::Page,
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
    SwitchPage(Page),
    // Profiles
    LoadProfiles,
    #[serde(skip)]
    ProfilesLoaded(Vec<ProfileInfo>),
    ProfilesLoadError(String),
    SelectNext,
    SelectPrevious,
    Confirm,
    ProfileSelected {
        name: String,
        region: Option<String>,
    },
    // S3
    LoadS3Buckets,
    #[serde(skip)]
    S3BucketsLoaded(Vec<BucketInfo>),
    S3BucketsError(String),
    // S3 Bucket
    LoadS3Objects {
        bucket_name: String,
    },
    #[serde(skip)]
    S3ObjectsLoaded(Vec<ObjectInfo>),
    S3ObjectsError(String),
}
