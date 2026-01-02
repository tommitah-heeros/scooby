use ratatui::{DefaultTerminal, Frame, crossterm};

pub struct Ui {}

impl Ui {
    pub fn run() -> color_eyre::Result<()> {
        let _ = color_eyre::install();
        ratatui::run(ui_application)?;
        Ok(())
    }
}

fn ui_application(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    loop {
        terminal.draw(render)?;
        if crossterm::event::read()?.is_key_press() {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    frame.render_widget("hello from ratatui", frame.area());
}
