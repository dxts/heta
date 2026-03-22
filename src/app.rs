use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Padding},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::{
    action::Action,
    aws::state::AwsState,
    components::{
        Component,
        common::{
            breadcrumb::Breadcrumb, command_bar::CommandBar, empty_area::EmptyArea, header::Header,
        },
        profiles::ProfilesList,
        s3_buckets::S3BucketsList,
        s3_objects::S3ObjectsList,
    },
    config::Config,
    page::Page,
    tui::{Event, Tui},
};

/// Central application struct. Owns all state, all components, and drives
/// the event → action → update → render loop.
pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    /// Buffer for multi-key combos (e.g. `gg`). Drained on every Tick.
    last_tick_key_events: Vec<KeyEvent>,
    /// Send side of the action channel — cloned into components so they
    /// can push actions back into the loop from anywhere (including async tasks).
    action_tx: mpsc::UnboundedSender<Action>,
    /// Receive side — drained every iteration in `handle_actions`.
    action_rx: mpsc::UnboundedReceiver<Action>,

    /// Shared AWS SDK state: config, clients, current profile/region.
    aws_state: AwsState,

    // ── Chrome: always-visible layout components ──
    header: Header,
    command_bar: CommandBar,
    breadcrumb: Breadcrumb,

    // ── Page: only one is active at a time ──
    active_page: Page,
    profiles_page: ProfilesList,
    s3_buckets_page: S3BucketsList,
    s3_objects_page: S3ObjectsList,
    empty_page: EmptyArea,
}

/// Input mode determines which keybinding set is active and whether
/// the command bar captures input.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Normal,
    Command,
    Filter,
}

impl App {
    /// Constructs the app: loads AWS config, builds the initial component
    /// tree, and kicks off the profiles view's async data fetch.
    /// Called once from `main` before entering the event loop.
    pub async fn new(tick_rate: f64, frame_rate: f64) -> color_eyre::Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let aws_state = AwsState::init().await?;

        let header = Header::new(&aws_state.profile, aws_state.region());

        let mut profiles_page = ProfilesList::default();
        profiles_page.register_action_handler(action_tx.clone())?;

        let mut s3_buckets_page = S3BucketsList::new(aws_state.s3_client.clone());
        s3_buckets_page.register_action_handler(action_tx.clone())?;

        let mut s3_objects_page = S3ObjectsList::new(aws_state.s3_client.clone());
        s3_objects_page.register_action_handler(action_tx.clone())?;

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
            breadcrumb: Breadcrumb::default(),
            active_page: Page::Profiles,
            profiles_page,
            s3_buckets_page,
            s3_objects_page,
            empty_page: EmptyArea::new(),
        })
    }

    /// Instead of repeatedly matching on `self.active_page` and dispatching
    /// the Component lifecycle calls to the correct component from App state
    ///
    /// This function will allow us to do:
    /// ```
    /// // In handle_key_event:
    /// let view_action = self.active_page_component().handle_key_event(key)?;
    ///
    /// // In handle_actions stage 3:
    /// let view_result = self.active_page_component().update(action.clone());
    ///
    /// // In render:
    /// let view_result = self.active_page_component().draw(frame, layout[2]);
    /// ```
    ///
    /// In the future, if you want to remove the dynamic dispatch and make it static
    /// this can be replaced by an enum wrapper pattern:
    /// ```
    /// enum PageComponent {
    ///     Profiles(ProfilesList),
    ///     S3Buckets(S3BucketsList),
    ///     S3Objects(S3ObjectsList),
    ///     Empty(EmptyArea),
    /// }
    ///
    /// impl Component for PageComponent {
    ///     fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    ///         match self {
    ///             Self::Profiles(c) => c.handle_key_event(key),
    ///             Self::S3Buckets(c) => c.handle_key_event(key),
    ///             Self::S3Objects(c) => c.handle_key_event(key),
    ///             Self::Empty(c) => c.handle_key_event(key),
    ///         }
    ///     }
    ///     // ... same delegation for update, draw, etc.
    /// }
    /// ```
    ///
    /// We can use the `enum_dispatch` crate to auto-generate this boilerplate:
    /// ```
    /// #[enum_dispatch(Component)]
    /// enum PageComponent { Profiles(ProfilesList), S3Buckets(S3BucketsList), ... }
    /// ```
    ///
    fn active_page_component(&mut self) -> &mut dyn Component {
        match self.active_page {
            Page::Profiles => &mut self.profiles_page,
            Page::S3Buckets => &mut self.s3_buckets_page,
            Page::S3Objects { .. } => &mut self.s3_objects_page,
            Page::Empty => &mut self.empty_page,
        }
    }

    /// The main event loop. Each iteration:
    ///   1. `handle_events` — poll terminal for input/tick/render events
    ///   2. `handle_actions` — drain the action queue, mutate state, render
    ///   3. Check for suspend/quit
    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut tui = Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui).await?;
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

    /// Phase 1 of the loop: reads one event from the terminal (blocking on
    /// the async event stream) and translates it into an `Action` on the queue.
    /// Key events go through `handle_key_event` for focus-aware routing.
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

    /// Focus-aware key routing. Priority order:
    ///   1. Command bar (when active — captures everything)
    ///   2. Active view (j/k/Enter etc. — returns Some to claim the key)
    ///   3. Global bindings (:, /, and config keymaps)
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        let action_tx = self.action_tx.clone();

        // 1. Command bar captures all input when active
        if self.command_bar.is_active() {
            if let Some(action) = self.command_bar.handle_key_event(key)? {
                action_tx.send(action)?;
            }
            return Ok(());
        }

        // 2. Active view gets first shot — returning Some claims the key
        let view_action = self.active_page_component().handle_key_event(key)?;
        if let Some(action) = view_action {
            action_tx.send(action)?;
            return Ok(());
        }

        // 3. Global keys and config-driven keybindings
        match key.code {
            KeyCode::Char(':') => {
                action_tx.send(Action::OpenCommandBar)?;
            }
            _ => {
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

    /// Phase 2 of the loop: drains the action queue and processes each action.
    /// Three stages per action:
    ///   1. App-level handling (quit, suspend, mode changes, view switching, AWS reload)
    ///   2. Propagate to chrome components (header, command bar, breadcrumb)
    ///   3. Propagate to the active view — follow-up actions are re-queued
    async fn handle_actions(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }

            // ── Stage 1: App-level state transitions ──
            match action.clone() {
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
                Action::CloseBar => self.mode = Mode::Normal,
                Action::SubmitCommand(ref cmd) => {
                    self.mode = Mode::Normal;
                    if let Some(view) = Page::from_command(cmd) {
                        self.action_tx.send(Action::SwitchPage(view))?;
                    } else {
                        self.action_tx
                            .send(Action::Error(format!("Unknown command: {cmd}")))?;
                    }
                }
                Action::SwitchPage(page) => {
                    self.breadcrumb
                        .set_segments(vec![self.aws_state.profile.clone(), page.label()]);
                    // Trigger the data loading action for the view
                    match page {
                        Page::Profiles => self.action_tx.send(Action::LoadProfiles)?,
                        Page::S3Buckets => self.action_tx.send(Action::LoadS3Buckets)?,
                        Page::S3Objects { ref bucket_name } => {
                            self.action_tx.send(Action::LoadS3Objects {
                                bucket_name: bucket_name.clone(),
                            })?
                        }
                        Page::Empty => (),
                    }
                    self.active_page = page;
                }
                Action::ProfileSelected {
                    ref name,
                    ref region,
                } => {
                    self.aws_state
                        .reload_for_profile(name, region.as_deref())
                        .await?;
                    self.header.set_profile(name);
                    self.header
                        .set_region(self.aws_state.region().unwrap_or("-"));
                    self.active_page = Page::Empty;
                }
                _ => {}
            }

            // ── Stage 2: Chrome components always see every action ──
            self.header.update(action.clone())?;
            self.command_bar.update(action.clone())?;
            self.breadcrumb.update(action.clone())?;

            // ── Stage 3: Only the active view receives the action ──
            let view_result = self.active_page_component().update(action.clone());
            if let Some(follow_up) = view_result? {
                self.action_tx.send(follow_up)?;
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> color_eyre::Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    /// Draws one frame. Layout is a vertical stack:
    ///   ┌─────────────────────┐
    ///   │ Header (5 lines)    │ profile/region + actions + logo + fps
    ///   ├─────────────────────┤
    ///   │ Command bar (0 or 3)│ shown only in Command/Filter mode
    ///   ├─────────────────────┤
    ///   │ Resource area (fill)│ delegates to the active view
    ///   ├─────────────────────┤
    ///   │ Breadcrumb (1 line) │ navigation trail
    ///   └─────────────────────┘
    fn render(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        tui.draw(|frame| {
            let area = frame.area();

            let bar_height = if self.command_bar.is_active() { 3 } else { 0 };

            let layout = Layout::vertical([
                Constraint::Length(5),
                Constraint::Length(bar_height),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

            // Header
            let header_block = Block::default().padding(Padding::horizontal(1));
            let header_inner = header_block.inner(layout[0]);
            frame.render_widget(header_block, layout[0]);
            if let Err(e) = self.header.draw(frame, header_inner) {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Header draw error: {e}")));
            }

            // Command/filter bar
            if self.command_bar.is_active()
                && let Err(e) = self.command_bar.draw(frame, layout[1])
            {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Bar draw error: {e}")));
            }

            // Active view
            let view_result = self.active_page_component().draw(frame, layout[2]);
            if let Err(e) = view_result {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("View draw error: {e}")));
            }

            // Breadcrumb
            let breadcrumb_block = Block::default().padding(Padding::horizontal(1));
            let breadcrumb_inner = breadcrumb_block.inner(layout[3]);
            frame.render_widget(breadcrumb_block, layout[3]);
            if let Err(e) = self.breadcrumb.draw(frame, breadcrumb_inner) {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Breadcrumb draw error: {e}")));
            }
        })?;
        Ok(())
    }
}
