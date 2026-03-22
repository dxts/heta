use aws_runtime::env_config::file::EnvConfigFiles;
use aws_types::os_shim_internal::{Env, Fs};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, components::Component};

pub struct ProfilesList {
    command_tx: Option<UnboundedSender<Action>>,
    profiles: Vec<ProfileInfo>,
    table_state: TableState,
    loading: bool,
}

impl Default for ProfilesList {
    fn default() -> Self {
        Self {
            command_tx: None,
            profiles: Vec::new(),
            table_state: TableState::default().with_selected(Some(0)),
            loading: true,
        }
    }
}

impl ProfilesList {
    fn selected_profile(&self) -> Option<&ProfileInfo> {
        self.table_state
            .selected()
            .and_then(|i| self.profiles.get(i))
    }

    fn select_next(&mut self) {
        if self.profiles.is_empty() {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let next = (current + 1).min(self.profiles.len() - 1);
        self.table_state.select(Some(next));
    }

    fn select_previous(&mut self) {
        let current = self.table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.table_state.select(Some(prev));
    }

    fn confirm(&self) -> Option<Action> {
        self.selected_profile().map(|p| Action::ProfileSelected {
            name: p.name.clone(),
            region: p.region.clone(),
        })
    }

    /// Spawns an async task to fetch profiles list. Results flow back
    /// as `ProfilesLoaded` or `ProfilesLoadError` through the action channel.
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
                self.select_next();
                Ok(None)
            }
            KeyCode::Up => {
                self.select_previous();
                Ok(None)
            }
            KeyCode::Enter => Ok(self.confirm()),
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::LoadProfiles => {
                self.loading = true;
                self.profiles.clear();
                self.spawn_load();
            }
            Action::ProfilesLoaded(profiles) => {
                self.profiles = profiles;
                self.loading = false;
                if !self.profiles.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            Action::SelectNext => self.select_next(),
            Action::SelectPrevious => self.select_previous(),
            Action::Confirm => return Ok(self.confirm()),
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Profiles ")
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        if self.loading {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                ratatui::widgets::Paragraph::new("Loading profiles...")
                    .style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            return Ok(());
        }

        let header_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let selected_style = Style::default()
            .bg(Color::DarkGray)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let normal_style = Style::default().fg(Color::Gray);

        let header =
            Row::new(vec![Cell::from("Profile"), Cell::from("Region")]).style(header_style);

        let rows: Vec<Row> = self
            .profiles
            .iter()
            .map(|p| {
                Row::new(vec![
                    Cell::from(p.name.as_str()),
                    Cell::from(p.region.as_deref().unwrap_or("-")),
                ])
                .style(normal_style)
            })
            .collect();

        let widths = [
            ratatui::layout::Constraint::Percentage(50),
            ratatui::layout::Constraint::Percentage(50),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .row_highlight_style(selected_style)
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(table, area, &mut self.table_state);

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////
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
