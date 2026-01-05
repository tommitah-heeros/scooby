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

enum FocusedWidget {
    List,
    Payload,
    Response,
}

/// Application state (which item is selected, etc.)
struct App {
    item_ids: Vec<String>,
    item_contents: HashMap<String, Option<serde_json::Value>>,
    item_responses: HashMap<String, Option<serde_json::Value>>,
    selected: usize,
    fullscreen: bool,
    payload_scroll: u16,
    response_scroll: u16,
    focused_widget: FocusedWidget,
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
            item_responses: display_data
                .iter()
                .map(|item| (item.key.clone(), item.response.clone()))
                .collect(),
            selected: 0,
            fullscreen: false,
            payload_scroll: 0,
            response_scroll: 0,
            focused_widget: FocusedWidget::List,
        }
    }

    fn next(&mut self) {
        if !self.item_ids.is_empty() && !self.fullscreen {
            self.selected = (self.selected + 1) % self.item_ids.len();
        }
    }

    fn previous(&mut self) {
        if !self.item_ids.is_empty() && !self.fullscreen {
            if self.selected == 0 {
                self.selected = self.item_ids.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
    }

    fn scroll_focused(&mut self, delta: i16) {
        match self.focused_widget {
            FocusedWidget::Payload => {
                let new = self.payload_scroll as i16 + delta;
                self.payload_scroll = new.max(0) as u16;
            }
            FocusedWidget::Response => {
                let new = self.response_scroll as i16 + delta;
                self.response_scroll = new.max(0) as u16;
            }
            // don't do anything for list
            _ => {}
        }
    }

    fn focus_next(&mut self) {
        if self.fullscreen {
            // in fullscreen, only Payload <-> Response are meaningful
            self.focused_widget = match self.focused_widget {
                FocusedWidget::Payload => FocusedWidget::Response,
                FocusedWidget::Response => FocusedWidget::Payload,
                // if fullscreen but focus is somehow on List, move to Payload.
                FocusedWidget::List => FocusedWidget::Payload,
            }
        }
        // in normal mode do nothing
    }
}

struct Grid<'a> {
    item_ids: &'a [String],
    item_contents: &'a HashMap<String, Option<serde_json::Value>>,
    item_responses: &'a HashMap<String, Option<serde_json::Value>>,
    selected: usize,
    fullscreen: bool,
    payload_scroll: u16,
    response_scroll: u16,
    focused_widget: &'a FocusedWidget,
}

impl Widget for Grid<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.fullscreen {
            // Fullscreen view: split the whole area into two vertical panes (left/right)
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(area);

            let left_area = layout[0];
            let right_area = layout[1];

            let payload_content = if self.item_ids.is_empty() {
                "No requests".to_string()
            } else {
                let key = &self.item_ids[self.selected];
                match self.item_contents.get(key) {
                    Some(Some(value)) => serde_json::to_string_pretty(value)
                        .unwrap_or_else(|_| "<invalid json>".into()),
                    Some(None) => "<no content>".into(),
                    None => "<missing content>".into(),
                }
            };

            let response_content = if self.item_ids.is_empty() {
                "No requests".to_string()
            } else {
                let key = &self.item_ids[self.selected];
                match self.item_responses.get(key) {
                    Some(Some(value)) => serde_json::to_string_pretty(value)
                        .unwrap_or_else(|_| "<invalid json>".into()),
                    Some(None) => "<no response>".into(),
                    None => "<missing response>".into(),
                }
            };

            let mut payload_block = Block::default().borders(Borders::ALL).title("Payload");
            let mut response_block = Block::default().borders(Borders::ALL).title("Response");
            if matches!(self.focused_widget, FocusedWidget::Payload) {
                payload_block = payload_block.border_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            }
            if matches!(self.focused_widget, FocusedWidget::Response) {
                response_block = response_block.border_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            }

            let payload_widget = Paragraph::new(payload_content)
                .block(payload_block)
                .scroll((self.payload_scroll, 0));
            let response_widget = Paragraph::new(response_content)
                .block(response_block)
                .scroll((self.response_scroll, 0));

            payload_widget.render(left_area, buf);
            response_widget.render(right_area, buf);
        } else {
            // Original grid view
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(area);

            let left_area = layout[0];
            let right_area = layout[1];

            let right_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(right_area);

            let right_top = right_layout[0];
            let right_bottom = right_layout[1];

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

            let list_block = Block::default().borders(Borders::ALL).title("Requests");
            let list = List::new(items).block(list_block);

            list.render(left_area, buf);

            let payload_content = if self.item_ids.is_empty() {
                "No requests".to_string()
            } else {
                let key = &self.item_ids[self.selected];
                match self.item_contents.get(key) {
                    Some(Some(value)) => serde_json::to_string_pretty(value)
                        .unwrap_or_else(|_| "<invalid json>".into()),
                    Some(None) => "<no content>".into(),
                    None => "<missing content>".into(),
                }
            };

            let payload_block = Block::default().borders(Borders::ALL).title("Payload");
            let payload_widget = Paragraph::new(payload_content)
                .block(payload_block)
                .scroll((self.payload_scroll, 0));

            payload_widget.render(right_top, buf);

            let response_content = if self.item_ids.is_empty() {
                "No requests".to_string()
            } else {
                let key = &self.item_ids[self.selected];
                match self.item_responses.get(key) {
                    Some(Some(value)) => serde_json::to_string_pretty(value)
                        .unwrap_or_else(|_| "<invalid json>".into()),
                    Some(None) => "<no response>".into(),
                    None => "<missing response>".into(),
                }
            };

            let response_block = Block::default().borders(Borders::ALL).title("Response");
            let response_widget = Paragraph::new(response_content)
                .block(response_block)
                .scroll((self.response_scroll, 0));

            response_widget.render(right_bottom, buf);
        }
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
                KeyCode::Enter => app.toggle_fullscreen(),
                KeyCode::Tab => app.focus_next(),
                KeyCode::Char('u')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    app.scroll_focused(-1)
                }
                KeyCode::Char('d')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    app.scroll_focused(1)
                }
                _ => {}
            }
        }
    }
}

fn render(frame: &mut Frame, app: &App) {
    let grid = Grid {
        item_ids: &app.item_ids,
        item_contents: &app.item_contents,
        item_responses: &app.item_responses,
        selected: app.selected,
        fullscreen: app.fullscreen,
        payload_scroll: app.payload_scroll,
        response_scroll: app.response_scroll,
        focused_widget: &app.focused_widget,
    };
    frame.render_widget(grid, frame.area());
}
