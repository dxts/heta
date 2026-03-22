use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    widgets::Clear,
};

use crate::action::Action;
use crate::components::Component;
use crate::components::common::resource_table::{ColumnDef, ResourceTable};
use crate::page::Page;

pub struct ResourceSelector {
    active: bool,
    table: ResourceTable<Page>,
}

impl Default for ResourceSelector {
    fn default() -> Self {
        let columns = vec![ColumnDef {
            header: "Resource",
            width: Constraint::Fill(1),
            cell: |page: &Page| page.label(),
        }];

        let mut table = ResourceTable::new("Resource Selector", columns);
        table.set_items(Page::selectable_pages());

        Self {
            active: false,
            table,
        }
    }
}

impl ResourceSelector {
    pub fn is_active(&self) -> bool {
        self.active
    }

    fn open(&mut self) {
        self.active = true;
        self.table.set_items(Page::selectable_pages());
    }

    fn close(&mut self) {
        self.active = false;
    }

    fn popup_area(area: Rect) -> Rect {
        let width = area.width * 40 / 100;
        let item_count = Page::selectable_pages().len() as u16;
        // header(1) + border top(1) + items + border bottom(1) + table header separator(1)
        let height = (item_count + 4).min(area.height - 2);

        let [popup_area] = Layout::horizontal([Constraint::Length(width)])
            .flex(Flex::Center)
            .areas(area);
        let [popup_area] = Layout::vertical([Constraint::Length(height)])
            .flex(Flex::Center)
            .areas(popup_area);

        popup_area
    }
}

impl Component for ResourceSelector {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if !self.active {
            return Ok(None);
        }

        match key.code {
            KeyCode::Esc => {
                self.close();
                Ok(Some(Action::CloseResourceSelector))
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.table.select_next();
                Ok(None)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.table.select_previous();
                Ok(None)
            }
            KeyCode::Enter => {
                if let Some(page) = self.table.selected() {
                    let page = page.clone();
                    self.close();
                    Ok(Some(Action::SwitchPage(page)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::OpenResourceSelector => self.open(),
            Action::CloseResourceSelector => self.close(),
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if !self.active {
            return Ok(());
        }

        let popup_area = Self::popup_area(area);
        frame.render_widget(Clear, popup_area);
        self.table.draw(frame, popup_area);

        Ok(())
    }
}
