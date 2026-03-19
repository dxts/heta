use serde::{Deserialize, Serialize};
use strum::Display;

use crate::aws::profiles::ProfileInfo;

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
    // Profiles
    #[serde(skip)]
    ProfilesLoaded(Vec<ProfileInfo>),
    SelectNext,
    SelectPrevious,
    Confirm,
    #[serde(skip)]
    ProfileSelected { name: String, region: Option<String> },
}