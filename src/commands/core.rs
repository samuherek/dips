use crate::tui;
use color_eyre::eyre::WrapErr;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::text::{Text};
use ratatui::widgets::{Block, Paragraph};

#[derive(Debug, Default)]
enum View {
    #[default]
    Entry,
}

#[derive(Debug, Default)]
struct App {
    view: View,
    exit: bool,
}

fn render(terminal: &mut tui::Tui, app: &mut App) -> color_eyre::Result<()> {
    terminal.draw(|frame| {
        frame.render_widget(
            Paragraph::new(Text::from("hellow")).block(Block::new()),
            frame.size(),
        );
    })?;
    Ok(())
}

/// Handles all the events
fn handle_events(app: &mut App) -> color_eyre::Result<()> {
    match event::read()? {
        // it's important to check that the event is a key press event as
        // crossterm also emits key release and repeat events on Windows.
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
            handle_key_event(key_event, app)
                .wrap_err_with(|| format!("handling key event failed: \n{key_event:#?}"))
        }
        _ => Ok(()),
    }
}

/// Handles all the key events
fn handle_key_event(event: KeyEvent, app: &mut App) -> color_eyre::Result<()> {
    match event.code {
        KeyCode::Esc => app.exit(),
        // KeyCode::Char(c) => app.enter_char(c),
        // KeyCode::Backspace => app.delete_char(),
        _ => {}
    }
    Ok(())
}

impl App {
    pub fn run(&mut self, terminal: &mut tui::Tui) -> color_eyre::Result<()> {
        while !self.exit {
            let _ = render(terminal, self);
            let _ = handle_events(self).wrap_err("failed to handle events")?;
        }
        Ok(())
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}

pub fn exec() -> color_eyre::Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let _ = App::default().run(&mut terminal);
    tui::restore()?;
    Ok(())
}
