use serde::{Deserialize, Serialize};

/// Define all pages (eg S3 Buckets, S3 Objects within a bucket, Lambda Functions, Profiles, ...)
/// Some pages can be selected using the command bar, like `:s3` which opens the S3Buckets page
/// Other pages are opened through parent pages,
/// like S3Objects can be opened by selecting a bucket on the S3Buckets pages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Page {
    Profiles,
    S3Buckets,
    S3Objects { bucket_name: String },
    Empty,
}

impl Page {
    pub fn from_command(cmd: &str) -> Option<Self> {
        match cmd.trim().to_lowercase().as_str() {
            "profiles" | "profile" | "p" => Some(Self::Profiles),
            "s3" | "s3buckets" | "buckets" => Some(Self::S3Buckets),
            "empty" | "e" => Some(Self::Empty),
            _ => None,
        }
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
