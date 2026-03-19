use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::action::Action;
use crate::components::Component;

#[derive(Default)]
pub struct EmptyArea;

impl EmptyArea {
    pub fn new() -> Self {
        Self
    }
}

impl Component for EmptyArea {
    fn update(&mut self, _action: Action) -> color_eyre::Result<Option<Action>> {
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Resources ")
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(
            Paragraph::new("No view selected. Press : to open command bar.")
                .style(Style::default().fg(Color::DarkGray))
                .block(block),
            area,
        );

        Ok(())
    }
}
