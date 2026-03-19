use serde::{Deserialize, Serialize};

/// Define all resource types (eg S3 Buckets, Lambda Functions, Profiles, ...)
/// which can be selected using the command bar like `:s3` etc
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Profiles,
    Empty,
}

impl ResourceType {
    pub fn from_command(cmd: &str) -> Option<Self> {
        match cmd.trim().to_lowercase().as_str() {
            "profiles" | "profile" | "p" => Some(Self::Profiles),
            "empty" | "e" => Some(Self::Empty),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Profiles => "profiles",
            Self::Empty => "empty",
        }
    }
}
