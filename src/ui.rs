use std::collections::HashMap;

use crate::db::{Db, to_ui_displayable};
use ratatui::layout::{Constraint, Direction};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{List, ListItem, Widget};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    crossterm::{self, event::KeyCode},
    layout::{Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};

pub struct Ui {}

impl Ui {
    // db not abstracted so not very kosher, this should probably work through a common interface
    // in reality
    pub fn run(db: &Db) -> color_eyre::Result<()> {
        let _ = color_eyre::install();
        ratatui::run(|terminal| ui_application(terminal, db))?;
        Ok(())
    }
}

/// Application state (which item is selected, etc.)
struct App {
    item_ids: Vec<String>,
    item_contents: HashMap<String, Option<serde_json::Value>>,
    selected: usize,
}

impl App {
    async fn new(db: &Db) -> Self {
        let data = match db.get_all_entries().await {
            Ok(data) => data,
            Err(err) => panic!("No data to be rendered: {}", err),
        };

        let display_data = to_ui_displayable(data);

        Self {
            item_ids: display_data.iter().map(|item| item.key.clone()).collect(),
            item_contents: display_data
                .iter()
                .map(|item| (item.key.clone(), item.content.clone()))
                .collect(),
            selected: 0,
        }
    }

    fn next(&mut self) {
        if !self.item_ids.is_empty() {
            self.selected = (self.selected + 1) % self.item_ids.len();
        }
    }

    fn previous(&mut self) {
        if !self.item_ids.is_empty() {
            if self.selected == 0 {
                self.selected = self.item_ids.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }
}

struct Grid<'a> {
    item_ids: &'a [String],
    item_contents: &'a HashMap<String, Option<serde_json::Value>>,
    selected: usize,
}

impl Widget for Grid<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(area);

        let left_area = layout[0];
        let right_area = layout[1];

        let items: Vec<ListItem> = self
            .item_ids
            .iter()
            .enumerate()
            .map(|(idx, text)| {
                let style = if idx == self.selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(text.clone()).style(style)
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Requests"));

        list.render(left_area, buf);

        // Determine content for the selected item
        let right_content = if self.item_ids.is_empty() {
            "No requests".to_string()
        } else {
            let key = &self.item_ids[self.selected];
            match self.item_contents.get(key) {
                Some(Some(value)) => {
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| "<invalid json>".into())
                }
                Some(None) => "<no content>".into(),
                None => "<missing content>".into(),
            }
        };

        let right = Paragraph::new(right_content)
            .block(Block::default().borders(Borders::ALL).title("Payload"));

        right.render(right_area, buf);
    }
}

fn ui_application(terminal: &mut DefaultTerminal, db: &Db) -> std::io::Result<()> {
    // this is VERY hacky, but can't be bothered with the async await nonsense right now. Passing
    // the db like this by reference isn't terribly smart as it is.
    let mut app =
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(App::new(db)));

    loop {
        terminal.draw(|frame| render(frame, &app))?;

        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            match key.code {
                KeyCode::Char('q') => break Ok(()),
                KeyCode::Char('j') => app.next(),
                KeyCode::Char('k') => app.previous(),
                _ => {}
            }
        }
    }
}

fn render(frame: &mut Frame, app: &App) {
    let grid = Grid {
        item_ids: &app.item_ids,
        item_contents: &app.item_contents,
        selected: app.selected,
    };
    frame.render_widget(grid, frame.area());
}
