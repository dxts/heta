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
    utils::pretty_bytes,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectInfo {
    pub name: String,
    pub size: Option<String>,
    pub last_modified: Option<i64>,
    pub etag: Option<String>,
}

pub struct S3ObjectsList {
    command_tx: Option<UnboundedSender<Action>>,
    client: Arc<RwLock<S3Client>>,
    bucket_name: String,
    table: ResourceTable<ObjectInfo>,
}

impl S3ObjectsList {
    pub fn new(client: Arc<RwLock<S3Client>>) -> Self {
        Self {
            command_tx: None,
            client,
            bucket_name: String::new(),
            table: ResourceTable::new(
                "S3 Objects",
                vec![
                    ColumnDef {
                        header: "Key",
                        width: Constraint::Percentage(50),
                        cell: |o| o.name.clone(),
                    },
                    ColumnDef {
                        header: "Size",
                        width: Constraint::Percentage(15),
                        cell: |o| o.size.clone().unwrap_or("-".into()),
                    },
                    ColumnDef {
                        header: "ETag",
                        width: Constraint::Percentage(35),
                        cell: |o| o.etag.clone().unwrap_or("-".into()),
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
                self.table.select_next();
                Ok(None)
            }
            KeyCode::Up => {
                self.table.select_previous();
                Ok(None)
            }
            KeyCode::Char('r') => {
                self.table.set_loading(true);
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
                self.table.set_title(format!("s3://{}", self.bucket_name));
                self.table.set_loading(true);
                self.spawn_load();
            }
            Action::S3ObjectsLoaded(objects) => {
                self.table.set_items(objects);
            }
            Action::S3ObjectsError(ref msg) => {
                self.table.set_loading(false);
                tracing::error!("S3 objects list error: {msg}");
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
////////////////////////////////////////////////////////////////////////////////

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
            size: o.size().map(|n| pretty_bytes(n as f64)),
            last_modified: o.last_modified().map(|t| t.to_millis().unwrap_or(0)),
            etag: o.e_tag().map(|s| s.to_string()),
        })
        .collect();
    Ok(objects)
}
