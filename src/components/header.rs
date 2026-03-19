use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::Component;
use crate::{action::Action, components::fps::FpsCounter};

pub struct Header {
    profile: String,
    region: String,
    account: String,
    // Context-dependent actions shown in the header
    context_actions: Vec<(String, String)>, // (label, key)
    fps_counter: FpsCounter,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            profile: "default".into(),
            region: "—".into(),
            account: "—".into(),
            context_actions: Vec::new(),
            fps_counter: FpsCounter::default(),
        }
    }
}

impl Header {
    pub fn new(profile: &str, region: Option<&str>) -> Self {
        Self {
            profile: profile.into(),
            region: region.unwrap_or("—").into(),
            ..Default::default()
        }
    }

    pub fn set_profile(&mut self, profile: &str) {
        self.profile = profile.to_string();
    }

    pub fn set_region(&mut self, region: &str) {
        self.region = region.to_string();
    }

    pub fn set_context_actions(&mut self, actions: Vec<(String, String)>) {
        self.context_actions = actions;
    }

    fn render_info_column(&self) -> Vec<Line<'_>> {
        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);

        vec![
            Line::from(vec![
                Span::styled("profile ", label_style),
                Span::styled(self.profile.as_str(), value_style),
            ]),
            Line::from(vec![
                Span::styled("region  ", label_style),
                Span::styled(self.region.as_str(), value_style),
            ]),
            Line::from(vec![
                Span::styled("account ", label_style),
                Span::styled(self.account.as_str(), value_style),
            ]),
        ]
    }

    fn render_actions_column(
        actions: &[(String, String)],
        offset: usize,
        count: usize,
    ) -> Vec<Line<'_>> {
        let key_style = Style::default().fg(Color::Yellow);
        let label_style = Style::default().fg(Color::Gray);

        actions
            .iter()
            .skip(offset)
            .take(count)
            .map(|(label, key)| {
                Line::from(vec![
                    Span::styled(format!("{:<10}", label), label_style),
                    Span::styled(format!(" {}", key), key_style),
                ])
            })
            .collect()
    }

    fn render_logo() -> Vec<Line<'static>> {
        let logo_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        vec![
            Line::from(Span::styled("HETA", logo_style)),
            Line::from(Span::styled(
                "aws tui",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    }
}

impl Component for Header {
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        self.fps_counter.update(action.clone())?;
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let columns = Layout::horizontal([
            Constraint::Percentage(25), // info
            Constraint::Percentage(25), // actions col 1
            Constraint::Percentage(25), // actions col 2
            Constraint::Percentage(25), // logo
        ])
        .split(area);

        // Info column
        let info_lines = self.render_info_column();
        frame.render_widget(Paragraph::new(info_lines), columns[0]);

        // Actions column 1 (first 3 actions)
        let col1_lines = Self::render_actions_column(&self.context_actions, 0, 3);
        frame.render_widget(Paragraph::new(col1_lines), columns[1]);

        // Actions column 2 (next 3 actions)
        let col2_lines = Self::render_actions_column(&self.context_actions, 3, 3);
        frame.render_widget(Paragraph::new(col2_lines), columns[2]);

        // Logo column: logo at top, FPS at bottom
        let logo_col = columns[3];
        let logo_rows = Layout::vertical([
            Constraint::Min(1),      // logo
            Constraint::Length(1),   // fps
        ])
        .split(logo_col);

        let logo_lines = Self::render_logo();
        frame.render_widget(
            Paragraph::new(logo_lines).alignment(ratatui::layout::Alignment::Right),
            logo_rows[0],
        );

        self.fps_counter.draw(frame, logo_rows[1])?;

        Ok(())
    }
}
