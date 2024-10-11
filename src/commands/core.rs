use crate::configuration;
use crate::models::dip::{self, DipRowFull};
use crate::models::dir_context::{self, ContextScope, DirContext};
use crate::tui;
use color_eyre::eyre::WrapErr;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::palette::tailwind::SLATE;
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    Wrap,
};
use sqlx::{Pool, Sqlite};

#[derive(Debug, Default)]
enum View {
    #[default]
    ContextList,
}

#[derive(Debug, Default)]
struct ContextListView {
    items: Vec<DipRowFull>,
    item_index: usize,
}

impl ContextListView {
    pub fn build(items: Vec<DipRowFull>) -> Self {
        Self {
            items,
            item_index: 0,
        }
    }

    /// Select the previous email (with wrap around).
    pub fn prev(&mut self) {
        if self.items.len() == 0 {
            return;
        }
        self.item_index = self.item_index.saturating_add(self.items.len() - 1) % self.items.len();
    }

    /// Select the next email (with wrap around).
    pub fn next(&mut self) {
        if self.items.len() == 0 {
            return;
        }
        self.item_index = self.item_index.saturating_add(1) % self.items.len();
    }
}

impl Widget for &ContextListView {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let items = self
            .items
            .iter()
            .map(|x| ListItem::new(x.value.clone()))
            .collect::<Vec<_>>();
        let list = List::new(items)
            .block(Block::new())
            .highlight_style(Style::new().bg(SLATE.c800))
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        let mut state = ListState::default().with_selected(Some(self.item_index));
        StatefulWidget::render(list, area, buf, &mut state);
    }
}

#[derive(Debug, Default, PartialEq)]
enum Mode {
    #[default]
    Running,
    Quit,
}

#[derive(Debug)]
struct App {
    db_pool: Pool<Sqlite>,
    mode: Mode,
    view: View,
    context_list_view: ContextListView,
    context_scope: ContextScope,
}

// - find the parent git -> get remote
// - compare db git remote compare the dir path

impl App {
    pub async fn build(config: configuration::Application) -> color_eyre::Result<Self> {
        let context_scope = dir_context::get_closest(&config.db_pool, &config.context_dir)
            .await
            .expect("Failed to get dir context");

        // let dir_context = dir_context::db_find_one(
        //     &config.db_pool,
        //     &config.context_dir.path(),
        //     config.context_dir.git_dir(),
        //     config.context_dir.git_remote(),
        // )
        // .await
        // .expect("Failed to find context dir");

        let items = dip::get_dir_context_all(&config.db_pool, &context_scope).await?;
        let context_list_view = ContextListView::build(items);
        Ok(Self {
            db_pool: config.db_pool,
            mode: Mode::default(),
            view: View::default(),
            context_list_view,
            context_scope,
        })
    }

    pub fn run(&mut self, terminal: &mut tui::Tui) -> color_eyre::Result<()> {
        while self.is_running() {
            terminal
                .draw(|frame| self.draw(frame))
                .wrap_err("terminal.draw")?;
            let _ = self.handle_events().wrap_err("failed to handle events")?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut ratatui::Frame) {
        frame.render_widget(self, frame.size());
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
            KeyCode::Esc => self.mode = Mode::Quit,
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            _ => {}
        }
        Ok(())
    }

    fn next(&mut self) {
        match self.view {
            View::ContextList => self.context_list_view.next(),
        }
    }

    fn prev(&mut self) {
        match self.view {
            View::ContextList => self.context_list_view.prev(),
        }
    }

    fn is_running(&self) -> bool {
        self.mode == Mode::Running
    }
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let layout = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Length(2),
                Constraint::Min(2),
                Constraint::Length(1),
            ],
        );
        let [header, main, toolbar] = layout.areas(area);
        self.render_header(header, buf);
        self.render_view(main, buf);
        self.render_toolbar(toolbar, buf);
    }
}

impl App {
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let text = Line::from(vec![
            Span::raw("Dips: "),
            Span::raw(self.context_scope.label()),
        ]);
        Paragraph::new(text)
            .block(
                Block::new()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::new().fg(SLATE.c500)),
            )
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }

    fn render_toolbar(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Text::from("<Esc> to exit"))
            .block(Block::new())
            .render(area, buf);
    }

    fn render_view(&self, area: Rect, buf: &mut Buffer) {
        match self.view {
            View::ContextList => self.context_list_view.render(area, buf),
        }
    }
}

pub async fn exec(config: configuration::Application) -> color_eyre::Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    // let list = dip::get_all(&config.db_pool).await?;
    let mut app = App::build(config).await?;
    // app.set_data(list);
    if let Err(e) = app.run(&mut terminal) {
        println!("{e:?}");
    }
    tui::restore()?;
    Ok(())
}
