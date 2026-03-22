use std::sync::Arc;

use aws_sdk_s3::Client as S3Client;
use crossterm::event::{KeyCode, KeyEvent};
use jiff::Timestamp;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use tokio::sync::{RwLock, mpsc::UnboundedSender};

use crate::{action::Action, components::Component, utils::pretty_bytes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectInfo {
    pub name: String,
    pub size: Option<i64>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

pub struct S3ObjectsList {
    command_tx: Option<UnboundedSender<Action>>,
    client: Arc<RwLock<S3Client>>,
    bucket_name: String,
    objects: Vec<ObjectInfo>,
    table_state: TableState,
    loading: bool,
}

impl S3ObjectsList {
    pub fn new(client: Arc<RwLock<S3Client>>) -> Self {
        Self {
            command_tx: None,
            client,
            bucket_name: String::new(),
            objects: Vec::new(),
            table_state: TableState::default().with_selected(Some(0)),
            loading: true,
        }
    }

    fn select_next(&mut self) {
        if self.objects.is_empty() {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let next = (current + 1).min(self.objects.len() - 1);
        self.table_state.select(Some(next));
    }

    fn select_previous(&mut self) {
        let current = self.table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.table_state.select(Some(prev));
    }

    fn spawn_load(&self) {
        let Some(tx) = self.command_tx.clone() else {
            return;
        };
        let client_lock = self.client.clone();
        let bucket_name = self.bucket_name.clone();
        tokio::spawn(async move {
            let client = client_lock.read().await.clone();
            match list_objects(&client, bucket_name).await {
                Ok(objects) => {
                    let _ = tx.send(Action::S3ObjectsLoaded(objects));
                }
                Err(e) => {
                    let _ = tx.send(Action::S3ObjectsError(e.to_string()));
                }
            }
        });
    }
}

impl Component for S3ObjectsList {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.command_tx = Some(tx);
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
            KeyCode::Char('r') => {
                self.loading = true;
                self.objects.clear();
                Ok(Some(Action::LoadS3Objects {
                    bucket_name: self.bucket_name.clone(),
                }))
            }
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::LoadS3Objects { ref bucket_name } => {
                self.bucket_name = bucket_name.clone();
                self.loading = true;
                self.objects.clear();
                self.spawn_load();
            }
            Action::S3ObjectsLoaded(objects) => {
                self.objects = objects;
                self.loading = false;
                if !self.objects.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            Action::S3ObjectsError(ref msg) => {
                self.loading = false;
                tracing::error!("S3 objects list error: {msg}");
            }
            Action::SelectNext => self.select_next(),
            Action::SelectPrevious => self.select_previous(),
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let title = format!(" s3://{} ", self.bucket_name);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        if self.loading {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                ratatui::widgets::Paragraph::new("Loading objects...")
                    .style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            return Ok(());
        }

        if self.objects.is_empty() {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                ratatui::widgets::Paragraph::new("No objects found")
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

        let header = Row::new(vec![
            Cell::from("Key"),
            Cell::from("Size"),
            Cell::from("Last Updated"),
            Cell::from("ETag"),
        ])
        .style(header_style);

        let rows: Vec<Row> = self
            .objects
            .iter()
            .map(|o| {
                let size_str = o
                    .size
                    .map(|s| pretty_bytes(s as f64))
                    .unwrap_or("-".to_string());
                Row::new(vec![
                    Cell::from(o.name.as_str()),
                    Cell::from(size_str),
                    Cell::from(o.last_modified.as_deref().unwrap_or("-")),
                    Cell::from(o.etag.as_deref().unwrap_or("-")),
                ])
                .style(normal_style)
            })
            .collect();

        let widths = [
            ratatui::layout::Constraint::Percentage(50),
            ratatui::layout::Constraint::Percentage(15),
            ratatui::layout::Constraint::Percentage(20),
            ratatui::layout::Constraint::Percentage(15),
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

/// Fetches objects in an S3 bucket.
pub async fn list_objects(
    client: &S3Client,
    bucket_name: String,
) -> color_eyre::Result<Vec<ObjectInfo>> {
    let resp = client.list_objects_v2().bucket(bucket_name).send().await?;

    let objects = resp
        .contents()
        .iter()
        .map(|o| ObjectInfo {
            name: o.key().unwrap_or("-").to_string(),
            size: o.size(),
            last_modified: o.last_modified().map(|t| {
                Timestamp::from_nanosecond(t.as_nanos())
                    .map(|t| t.to_string())
                    .unwrap_or("malformed".to_string())
            }),
            etag: o.e_tag().map(|s| s.to_string()),
        })
        .collect();

    Ok(objects)
}
