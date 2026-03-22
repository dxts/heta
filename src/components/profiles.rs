use aws_runtime::env_config::file::EnvConfigFiles;
use aws_types::os_shim_internal::{Env, Fs};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    components::{
        Component,
        common::resource_table::{ColumnDef, ResourceTable},
    },
};

pub struct ProfilesList {
    command_tx: Option<UnboundedSender<Action>>,
    table: ResourceTable<ProfileInfo>,
}

impl Default for ProfilesList {
    fn default() -> Self {
        Self {
            command_tx: None,
            table: ResourceTable::new(
                "Profiles",
                vec![
                    ColumnDef {
                        header: "Profile",
                        width: Constraint::Percentage(50),
                        cell: |p| p.name.clone(),
                    },
                    ColumnDef {
                        header: "Region",
                        width: Constraint::Percentage(50),
                        cell: |p| p.region.clone().unwrap_or("-".into()),
                    },
                ],
            ),
        }
    }
}

impl ProfilesList {
    fn confirm(&self) -> Option<Action> {
        self.table.selected().map(|p| Action::ProfileSelected {
            name: p.name.clone(),
            region: p.region.clone(),
        })
    }

    fn spawn_load(&self) {
        let Some(tx) = self.command_tx.clone() else {
            return;
        };
        tokio::spawn(async move {
            match list_profiles().await {
                Ok(profiles) => {
                    let _ = tx.send(Action::ProfilesLoaded(profiles));
                }
                Err(e) => {
                    let _ = tx.send(Action::ProfilesLoadError(e.to_string()));
                }
            }
        });
    }
}

impl Component for ProfilesList {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.command_tx = Some(tx.clone());
        // Kick off initial load
        tokio::spawn(async move {
            match list_profiles().await {
                Ok(profiles) => {
                    let _ = tx.send(Action::ProfilesLoaded(profiles));
                }
                Err(e) => {
                    let _ = tx.send(Action::Error(format!("Failed to load profiles: {e}")));
                }
            }
        });
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        match key.code {
            KeyCode::Down => {
                self.table.select_next();
                Ok(None)
            }
            KeyCode::Up => {
                self.table.select_previous();
                Ok(None)
            }
            KeyCode::Enter => Ok(self.confirm()),
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::LoadProfiles => {
                self.table.set_loading(true);
                self.spawn_load();
            }
            Action::ProfilesLoaded(profiles) => {
                self.table.set_items(profiles);
            }
            Action::SelectNext => self.table.select_next(),
            Action::SelectPrevious => self.table.select_previous(),
            Action::Confirm => return Ok(self.confirm()),
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        self.table.draw(frame, area);
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileInfo {
    pub name: String,
    pub region: Option<String>,
}

pub async fn list_profiles() -> color_eyre::Result<Vec<ProfileInfo>> {
    let env_config =
        aws_config::profile::load(&Fs::real(), &Env::real(), &EnvConfigFiles::default(), None)
            .await?;

    let mut profiles: Vec<ProfileInfo> = env_config
        .profiles()
        .map(|name| {
            let profile = env_config.get_profile(name);
            ProfileInfo {
                name: name.to_string(),
                region: profile.and_then(|p| p.get("region")).map(|s| s.to_string()),
            }
        })
        .collect();

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}
