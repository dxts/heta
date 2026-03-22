use aws_sdk_s3::Client as S3Client;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, components::Component};

pub struct S3BucketsList {
    command_tx: Option<UnboundedSender<Action>>,
    client: S3Client,
    buckets: Vec<BucketInfo>,
    table_state: TableState,
    loading: bool,
}

impl S3BucketsList {
    pub fn new(client: S3Client) -> Self {
        Self {
            command_tx: None,
            client,
            buckets: Vec::new(),
            table_state: TableState::default().with_selected(Some(0)),
            loading: true,
        }
    }

    fn select_next(&mut self) {
        if self.buckets.is_empty() {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let next = (current + 1).min(self.buckets.len() - 1);
        self.table_state.select(Some(next));
    }

    fn select_previous(&mut self) {
        let current = self.table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.table_state.select(Some(prev));
    }

    /// Spawns an async task to fetch bucket list. Results flow back
    /// as `S3BucketsLoaded` or `S3BucketsError` through the action channel.
    fn spawn_load(&self) {
        let Some(tx) = self.command_tx.clone() else {
            return;
        };

        let client = self.client.clone();
        tokio::spawn(async move {
            match list_buckets(&client).await {
                Ok(buckets) => {
                    let _ = tx.send(Action::S3BucketsLoaded(buckets));
                }
                Err(e) => {
                    let _ = tx.send(Action::S3BucketsError(e.to_string()));
                }
            }
        });
    }
}

impl Component for S3BucketsList {
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
                self.buckets.clear();
                Ok(Some(Action::LoadS3Buckets))
            }
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::LoadS3Buckets => {
                self.loading = true;
                self.buckets.clear();
                self.spawn_load();
            }
            Action::S3BucketsLoaded(buckets) => {
                self.buckets = buckets;
                self.loading = false;
                if !self.buckets.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            Action::S3BucketsError(ref msg) => {
                self.loading = false;
                tracing::error!("S3 bucket list error: {msg}");
            }
            Action::SelectNext => self.select_next(),
            Action::SelectPrevious => self.select_previous(),
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" S3 Buckets ")
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        if self.loading {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                ratatui::widgets::Paragraph::new("Loading S3 buckets...")
                    .style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            return Ok(());
        }

        if self.buckets.is_empty() {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                ratatui::widgets::Paragraph::new("No buckets found")
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
            Row::new(vec![Cell::from("Bucket"), Cell::from("Created")]).style(header_style);

        let rows: Vec<Row> = self
            .buckets
            .iter()
            .map(|b| {
                Row::new(vec![
                    Cell::from(b.name.as_str()),
                    Cell::from(b.creation_date.as_deref().unwrap_or("—")),
                ])
                .style(normal_style)
            })
            .collect();

        let widths = [
            ratatui::layout::Constraint::Percentage(60),
            ratatui::layout::Constraint::Percentage(40),
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
pub struct BucketInfo {
    pub name: String,
    pub region: Option<String>,
    pub creation_date: Option<String>,
}

/// Fetches all S3 buckets visible to the current credentials.
pub async fn list_buckets(client: &S3Client) -> color_eyre::Result<Vec<BucketInfo>> {
    let resp = client.list_buckets().send().await?;

    let buckets = resp
        .buckets()
        .iter()
        .map(|b| BucketInfo {
            name: b.name().unwrap_or("—").to_string(),
            region: None, // bucket-level region requires a per-bucket HEAD call
            creation_date: b.creation_date().map(|d| {
                d.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime)
                    .unwrap_or_default()
            }),
        })
        .collect();

    Ok(buckets)
}
