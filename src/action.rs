use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{aws::profiles::ProfileInfo, resource_selector::ResourceType};

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
    OpenFilterBar,
    CloseBar,
    SubmitCommand(String),
    SubmitFilter(String),
    // Navigation
    SwitchView(ResourceType),
    // Profiles
    #[serde(skip)]
    ProfilesLoaded(Vec<ProfileInfo>),
    SelectNext,
    SelectPrevious,
    Confirm,
    #[serde(skip)]
    ProfileSelected {
        name: String,
        region: Option<String>,
    },
}
