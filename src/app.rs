use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::{
    action::Action,
    aws::state::AwsState,
    components::{
        Component,
        breadcrumb::Breadcrumb,
        command_bar::CommandBar,
        header::Header,
        home::Home,
    },
    config::Config,
    tui::{Event, Tui},
};

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
    aws_state: AwsState,

    // Layout components
    header: Header,
    command_bar: CommandBar,
    resource_area: Home,
    breadcrumb: Breadcrumb,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Normal,
    Command,
    Filter,
}

impl App {
    pub async fn new(tick_rate: f64, frame_rate: f64) -> color_eyre::Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let aws_state = AwsState::init().await?;

        let header = Header::new("default", aws_state.region());

        Ok(Self {
            tick_rate,
            frame_rate,
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            mode: Mode::Normal,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            aws_state,
            header,
            command_bar: CommandBar::default(),
            resource_area: Home::new(),
            breadcrumb: Breadcrumb::default(),
        })
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut tui = Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        let action_tx = self.action_tx.clone();

        // Command bar captures all input when active
        if self.command_bar.is_active() {
            if let Some(action) = self.command_bar.handle_key_event(key)? {
                action_tx.send(action)?;
            }
            return Ok(());
        }

        // Normal mode key handling
        match key.code {
            KeyCode::Char(':') => {
                action_tx.send(Action::OpenCommandBar)?;
            }
            KeyCode::Char('/') => {
                action_tx.send(Action::OpenFilterBar)?;
            }
            _ => {
                // Check keybindings config
                if let Some(keymap) = self.config.keybindings.0.get(&self.mode) {
                    match keymap.get(&vec![key]) {
                        Some(action) => {
                            info!("Got action: {action:?}");
                            action_tx.send(action.clone())?;
                        }
                        _ => {
                            self.last_tick_key_events.push(key);
                            if let Some(action) = keymap.get(&self.last_tick_key_events) {
                                info!("Got action: {action:?}");
                                action_tx.send(action.clone())?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }
            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::OpenCommandBar => self.mode = Mode::Command,
                Action::OpenFilterBar => self.mode = Mode::Filter,
                Action::CloseBar | Action::SubmitCommand(_) | Action::SubmitFilter(_) => {
                    self.mode = Mode::Normal;
                }
                _ => {}
            }

            // Propagate to all layout components
            for result in [
                self.header.update(action.clone()),
                self.command_bar.update(action.clone()),
                self.resource_area.update(action.clone()),
                self.breadcrumb.update(action.clone()),
            ] {
                if let Some(follow_up) = result? {
                    self.action_tx.send(follow_up)?;
                }
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> color_eyre::Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        tui.draw(|frame| {
            let area = frame.area();

            // Command bar gets 1 line when active, 0 when hidden
            let bar_height = if self.command_bar.is_active() { 1 } else { 0 };

            let layout = Layout::vertical([
                Constraint::Length(5),          // header
                Constraint::Length(bar_height), // command/filter bar
                Constraint::Min(1),             // resource area
                Constraint::Length(1),          // breadcrumb
            ])
            .split(area);

            // Header with border
            let header_block = Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray));
            let header_inner = header_block.inner(layout[0]);
            frame.render_widget(header_block, layout[0]);
            if let Err(e) = self.header.draw(frame, header_inner) {
                let _ = self.action_tx.send(Action::Error(format!("Header draw error: {e}")));
            }

            // Command/filter bar
            if self.command_bar.is_active() && let Err(e) = self.command_bar.draw(frame, layout[1]) {
                let _ = self.action_tx.send(Action::Error(format!("Command bar draw error: {e}")));
            }

            // Resource area
            if let Err(e) = self.resource_area.draw(frame, layout[2]) {
                let _ = self.action_tx.send(Action::Error(format!("Resource area draw error: {e}")));
            }

            // Breadcrumb with top border
            let breadcrumb_block = Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray));
            let breadcrumb_inner = breadcrumb_block.inner(layout[3]);
            frame.render_widget(breadcrumb_block, layout[3]);
            if let Err(e) = self.breadcrumb.draw(frame, breadcrumb_inner) {
                let _ = self.action_tx.send(Action::Error(format!("Breadcrumb draw error: {e}")));
            }
        })?;
        Ok(())
    }
}