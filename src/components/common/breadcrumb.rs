use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::action::Action;
use crate::components::Component;

pub struct Breadcrumb {
    segments: Vec<String>,
}

impl Default for Breadcrumb {
    fn default() -> Self {
        Self {
            segments: vec!["home".into()],
        }
    }
}

impl Breadcrumb {
    pub fn set_segments(&mut self, segments: Vec<String>) {
        self.segments = segments;
    }
}

impl Component for Breadcrumb {
    fn update(&mut self, _action: Action) -> color_eyre::Result<Option<Action>> {
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let separator_style = Style::default().fg(Color::DarkGray);
        let segment_style = Style::default().fg(Color::Cyan);

        let mut spans: Vec<Span> = Vec::new();
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" › ", separator_style));
            }
            spans.push(Span::styled(segment.as_str(), segment_style));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);

        Ok(())
    }
}
