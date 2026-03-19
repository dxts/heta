use serde::{Deserialize, Serialize};
use strum::Display;

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
    ProfilesLoaded(Vec<String>),
    // Command bar
    OpenCommandBar,
    OpenFilterBar,
    CloseBar,
    SubmitCommand(String),
    SubmitFilter(String),
}