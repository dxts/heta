use rat_ftable::selection::RowSelection;
use rat_ftable::textdata::Row as FtRow;
use rat_ftable::{Table, TableContext, TableData, TableState};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

/// Defines how a column maps from data item `T` to display text.
pub struct ColumnDef<T> {
    pub header: &'static str,
    pub width: Constraint,
    pub cell: fn(&T) -> String,
}

/// Generic, reusable table component backed by `rat_ftable`.
/// Pages provide their data type `T`, column definitions, and data -
/// the table handles rendering, selection, scrolling, and loading/empty states.
pub struct ResourceTable<T> {
    title: String,
    columns: Vec<ColumnDef<T>>,
    items: Vec<T>,
    state: TableState<RowSelection>,
    loading: bool,
}

impl<T> ResourceTable<T> {
    pub fn new(title: impl Into<String>, columns: Vec<ColumnDef<T>>) -> Self {
        Self {
            title: title.into(),
            columns,
            items: Vec::new(),
            state: TableState::default(),
            loading: true,
        }
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.loading = false;
        if !self.items.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.clear_selection();
        }
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
        if loading {
            self.items.clear();
        }
    }

    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let next = (current + 1).min(self.items.len() - 1);
        self.state.select(Some(next));
    }

    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        self.state.select(Some(current.saturating_sub(1)));
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" {} ", self.title))
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        if self.loading {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                Paragraph::new("Loading...").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            return;
        }

        if self.items.is_empty() {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                Paragraph::new("No items found").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            return;
        }

        let bridge = TableBridge {
            items: &self.items,
            columns: &self.columns,
        };

        let table = Table::default()
            .data(bridge)
            .block(block)
            .select_row_style(Some(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
            .header_style(Some(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().fg(Color::Gray));

        table.render(area, frame.buffer_mut(), &mut self.state);
    }
}

/// Thin bridge that implements `TableData` by borrowing the items and column defs.
/// Only visible cells are rendered - no `Vec<Row>` allocation.
struct TableBridge<'a, T> {
    items: &'a [T],
    columns: &'a [ColumnDef<T>],
}

impl<'a, T> TableData<'a> for TableBridge<'a, T> {
    fn rows(&self) -> usize {
        self.items.len()
    }

    fn header(&self) -> Option<FtRow<'a>> {
        let cells: Vec<&str> = self.columns.iter().map(|c| c.header).collect();
        Some(FtRow::new(cells))
    }

    fn widths(&self) -> Vec<Constraint> {
        self.columns.iter().map(|c| c.width).collect()
    }

    fn render_cell(
        &self,
        _ctx: &TableContext,
        column: usize,
        row: usize,
        area: Rect,
        buf: &mut Buffer,
    ) {
        if let (Some(col_def), Some(item)) = (self.columns.get(column), self.items.get(row)) {
            let text = (col_def.cell)(item);
            Span::from(text).render(area, buf);
        }
    }
}
