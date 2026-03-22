use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Rect, Size},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, config::Config, tui::Event};

pub mod common;
pub mod profiles;
pub mod s3_buckets;

#[allow(unused)]
/// `Component` is a trait that represents a visual and interactive element of the user interface.
///
/// Implementors of this trait can be registered with the main application loop and will be able to
/// receive events, update state, and be rendered on the screen.
pub trait Component {
    /// Called once during app startup. Gives the component a channel to send
    /// actions back into the main loop - use this to trigger async work
    /// (e.g. spawning a tokio task that sends `S3BucketListLoaded` when done).
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        let _ = tx;
        Ok(())
    }

    /// Called once during app startup, after `register_action_handler`.
    /// Provides the deserialized app config so components can read
    /// keybindings, styles, or any user preferences they need.
    fn register_config_handler(&mut self, config: Config) -> color_eyre::Result<()> {
        let _ = config;
        Ok(())
    }

    /// Called once after config registration, with the initial terminal size.
    /// Use this for any setup that depends on knowing the drawable area
    /// (e.g. pre-computing layout or allocating buffers).
    fn init(&mut self, area: Size) -> color_eyre::Result<()> {
        let _ = area;
        Ok(())
    }

    /// Dispatches raw terminal events to the appropriate typed handler.
    /// Override this only if you need custom event routing; the default
    /// delegates to `handle_key_event` and `handle_mouse_event`.
    fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<Option<Action>> {
        let action = match event {
            Some(Event::Key(key_event)) => self.handle_key_event(key_event)?,
            Some(Event::Mouse(mouse_event)) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };
        Ok(action)
    }

    /// Called when a key is pressed and this component has input focus.
    /// Return `Some(Action)` to feed an action into the main loop,
    /// or `None` to let the event fall through to global keybindings.
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        let _ = key;
        Ok(None)
    }

    /// Called when a mouse event occurs and this component has focus.
    /// Same return semantics as `handle_key_event`.
    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> color_eyre::Result<Option<Action>> {
        let _ = mouse;
        Ok(None)
    }

    /// Called on every action dispatched through the main loop - including
    /// `Tick`, `Render`, and any custom actions. This is where components
    /// mutate their own state in response to actions. Returning `Some(Action)`
    /// chains a follow-up action back into the loop.
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        let _ = action;
        Ok(None)
    }

    /// Called on every frame when `Action::Render` fires. The component
    /// draws itself into the given `area` of the terminal. This is the
    /// only place rendering should happen - keep it side-effect free
    /// beyond writing to the frame.
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()>;
}
