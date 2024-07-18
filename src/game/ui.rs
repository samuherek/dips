use super::{app, tui, ui_utils};
use color_eyre::eyre::WrapErr;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::Alignment;
use ratatui::style::Stylize;
use ratatui::symbols::border;
use ratatui::text::{Line, Text};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{Block, Paragraph};

/// runs the application's main loop until the user quits
pub fn render(terminal: &mut tui::Tui, app: &mut app::App) -> color_eyre::Result<()> {
    terminal.draw(|frame| {
        let main_area = ui_utils::centered_rect(frame.size(), 75, 75);
        let input = Text::from(vec![Line::from(vec![app.input().into()])]);
        let main_block = {
            let title = Title::from(" Dips ".bold());
            let info = Title::from(Line::from(vec![" Quit ".into(), "<Esc> ".blue().bold()]));
            Block::bordered()
                .title(title.alignment(Alignment::Center))
                .title(info.alignment(Alignment::Center).position(Position::Bottom))
                .border_set(border::THICK)
        };
        let paragraph = Paragraph::new(input).block(main_block);
        frame.render_widget(paragraph, main_area);
        let (cursor_x, cursor_y) = app.input_cursor_position(main_area.x, main_area.y);
        frame.set_cursor(cursor_x, cursor_y);
    })?;

    Ok(())
}

/// Handles all the events
pub fn handle_events(app: &mut app::App) -> color_eyre::Result<()> {
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
fn handle_key_event(event: KeyEvent, app: &mut app::App) -> color_eyre::Result<()> {
    match event.code {
        KeyCode::Esc => app.exit(),
        KeyCode::Char(c) => app.enter_char(c),
        KeyCode::Backspace => app.delete_char(),
        _ => {}
    }
    Ok(())
}
