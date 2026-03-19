//! Views are also components but more complex ones
//! that usually correspond to the primary interactions with an aws resource.

use serde::{Deserialize, Serialize};

pub mod profiles;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViewTypes {
    Profiles,
    Home,
}

impl ViewTypes {
    pub fn from_command(cmd: &str) -> Option<Self> {
        match cmd.trim().to_lowercase().as_str() {
            "profiles" | "profile" | "p" => Some(Self::Profiles),
            "home" | "h" => Some(Self::Home),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Profiles => "profiles",
            Self::Home => "home",
        }
    }
}