use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::action::Action;
use crate::components::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarMode {
    Command,
    Hidden,
}

pub struct CommandBar {
    mode: BarMode,
    input: Input,
}

impl Default for CommandBar {
    fn default() -> Self {
        Self {
            mode: BarMode::Hidden,
            input: Input::default(),
        }
    }
}

impl CommandBar {
    pub fn is_active(&self) -> bool {
        self.mode != BarMode::Hidden
    }

    fn prefix(&self) -> &str {
        match self.mode {
            BarMode::Command => ":",
            BarMode::Hidden => "",
        }
    }

    fn open(&mut self, mode: BarMode) {
        self.mode = mode;
    }

    fn close(&mut self) {
        self.mode = BarMode::Hidden;
        self.input.reset();
    }

    fn submit(&mut self) -> Option<Action> {
        let action = match self.mode {
            BarMode::Command => Some(Action::SubmitCommand(self.input.value_and_reset())),
            BarMode::Hidden => None,
        };
        self.close();
        action
    }
}

impl Component for CommandBar {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if !self.is_active() {
            return Ok(None);
        }

        match key.code {
            KeyCode::Esc => {
                self.close();
                Ok(Some(Action::CloseBar))
            }
            KeyCode::Enter => Ok(self.submit()),
            _ => {
                let event = Event::Key(key);
                self.input.handle_event(&event);
                Ok(None)
            }
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::OpenCommandBar => self.open(BarMode::Command),
            Action::CloseBar => self.close(),
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if !self.is_active() {
            return Ok(());
        }

        let prefix_style = Style::default().fg(Color::DarkGray);
        let input_style = Style::default().fg(Color::DarkGray);

        let line = Line::from(vec![
            Span::styled(self.prefix(), prefix_style),
            Span::styled(self.input.value(), input_style),
        ]);

        let block = Block::bordered().border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(line).block(block), area);

        // Position cursor after the border + prefix + input
        let cursor_x = area.x + 1 + self.prefix().len() as u16 + self.input.visual_cursor() as u16;
        // Position after the border
        let cursor_y = area.y + 1;

        frame.set_cursor_position((cursor_x, cursor_y));

        Ok(())
    }
}
