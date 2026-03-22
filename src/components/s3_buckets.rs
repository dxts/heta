use std::sync::Arc;

use aws_sdk_s3::Client as S3Client;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
};
use tokio::sync::{RwLock, mpsc::UnboundedSender};

use crate::{
    action::Action,
    components::{
        Component,
        common::resource_table::{ColumnDef, ResourceTable},
    },
    page::Page,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketInfo {
    pub name: String,
    pub region: Option<String>,
    pub creation_date: Option<String>,
}

pub struct S3BucketsList {
    command_tx: Option<UnboundedSender<Action>>,
    client: Arc<RwLock<S3Client>>,
    table: ResourceTable<BucketInfo>,
}

impl S3BucketsList {
    pub fn new(client: Arc<RwLock<S3Client>>) -> Self {
        Self {
            command_tx: None,
            client,
            table: ResourceTable::new(
                "S3 Buckets",
                vec![
                    ColumnDef {
                        header: "Bucket",
                        width: Constraint::Percentage(60),
                        cell: |b| b.name.clone(),
                    },
                    ColumnDef {
                        header: "Created",
                        width: Constraint::Percentage(40),
                        cell: |b| b.creation_date.clone().unwrap_or("-".into()),
                    },
                ],
            ),
        }
    }

    fn spawn_load(&self) {
        let Some(tx) = self.command_tx.clone() else {
            return;
        };
        let client_lock = self.client.clone();
        tokio::spawn(async move {
            let client = client_lock.read().await.clone();
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
                self.table.select_next();
                Ok(None)
            }
            KeyCode::Up => {
                self.table.select_previous();
                Ok(None)
            }
            KeyCode::Enter => Ok(self.table.selected().map(|b| {
                Action::SwitchPage(Page::S3Objects {
                    bucket_name: b.name.clone(),
                })
            })),
            KeyCode::Char('r') => {
                self.table.set_loading(true);
                Ok(Some(Action::LoadS3Buckets))
            }
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::LoadS3Buckets => {
                self.table.set_loading(true);
                self.spawn_load();
            }
            Action::S3BucketsLoaded(buckets) => {
                self.table.set_items(buckets);
            }
            Action::S3BucketsError(ref msg) => {
                self.table.set_loading(false);
                tracing::error!("S3 bucket list error: {msg}");
            }
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

pub async fn list_buckets(client: &S3Client) -> color_eyre::Result<Vec<BucketInfo>> {
    let resp = client.list_buckets().send().await?;
    let buckets = resp
        .buckets()
        .iter()
        .map(|b| BucketInfo {
            name: b.name().unwrap_or("-").to_string(),
            region: None,
            creation_date: b.creation_date().map(|d| {
                d.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime)
                    .unwrap_or_default()
            }),
        })
        .collect();
    Ok(buckets)
}
