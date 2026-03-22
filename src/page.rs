use serde::{Deserialize, Serialize};

/// Define all pages (eg S3 Buckets, S3 Objects within a bucket, Lambda Functions, Profiles, ...)
/// Top-level pages can be selected from the resource selector popup (`:` key).
/// Other pages are opened through parent pages,
/// like S3Objects can be opened by selecting a bucket on the S3Buckets page.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Page {
    Profiles,
    S3Buckets,
    S3Objects { bucket_name: String },
    Empty,
}

impl Page {
    /// Top-level pages that can be selected from the resource selector popup.
    /// Excludes parameterized pages (like S3Objects) and Empty.
    pub fn selectable_pages() -> Vec<Self> {
        vec![Page::Profiles, Page::S3Buckets]
    }

    pub fn label(&self) -> String {
        match self {
            Self::Profiles => "profiles".to_string(),
            Self::S3Buckets => "s3".to_string(),
            Self::S3Objects { bucket_name } => bucket_name.clone(),
            Self::Empty => "empty".to_string(),
        }
    }
}
