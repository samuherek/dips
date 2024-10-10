use crate::configuration;
use crate::models::dip::{self, DipRowFull};
use crate::models::dir_context::RuntimeDirContext;
use crate::tui;
use color_eyre::eyre::WrapErr;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::palette::tailwind::SLATE;
use ratatui::style::{Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph};

#[derive(Debug, Default)]
enum View {
    #[default]
    Entry,
}

#[derive(Debug)]
struct App<'a> {
    view: View,
    dir_context: &'a RuntimeDirContext,
    items: Vec<DipRowFull>,
    list_state: ListState,
    exit: bool,
}

fn render(terminal: &mut tui::Tui, app: &mut App) -> color_eyre::Result<()> {
    terminal.draw(|frame| {
        let layout = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Length(2),
                Constraint::Min(2),
                Constraint::Length(1),
            ],
        )
        .split(frame.size());

        frame.render_widget(
            Paragraph::new(Text::from(app.dir_context.path()))
                .block(Block::new().borders(Borders::BOTTOM)),
            layout[0],
        );

        let items = app
            .items
            .iter()
            .map(|x| ListItem::new(x.value.clone()))
            .collect::<Vec<_>>();
        let list = List::new(items)
            .block(Block::new())
            .highlight_style(Style::new().bg(SLATE.c800))
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        // StatefulWidget::render(list, area, buf, &mut self.todo_list.state);
        frame.render_stateful_widget(list, layout[1], &mut app.list_state);

        frame.render_widget(
            Paragraph::new(Text::from("<Esc> to exit")).block(Block::new()),
            layout[2],
        );
    })?;
    Ok(())
}

impl<'a> App<'a> {
    pub fn new(dir_context: &'a RuntimeDirContext) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            view: View::default(),
            dir_context,
            items: Vec::default(),
            list_state,
            exit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut tui::Tui) -> color_eyre::Result<()> {
        while !self.exit {
            let _ = render(terminal, self);
            let _ = self.handle_events().wrap_err("failed to handle events")?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self
                .handle_key_event(key_event)
                .wrap_err_with(|| format!("handling key event failed: \n{key_event:#?}")),
            _ => Ok(()),
        }
    }

    /// Handles all the key events
    fn handle_key_event(&mut self, event: KeyEvent) -> color_eyre::Result<()> {
        match event.code {
            KeyCode::Esc => self.exit(),
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            _ => {}
        }
        Ok(())
    }

    // Add helper methods for navigation
    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn prev(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn set_data(&mut self, data: Vec<DipRowFull>) {
        self.items = data;
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}

pub async fn exec(config: &configuration::Application) -> color_eyre::Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let list = dip::get_all(&config.db_pool).await?;
    let mut app = App::new(&config.context_dir);
    app.set_data(list);
    let _ = app.run(&mut terminal);
    tui::restore()?;
    Ok(())
}
