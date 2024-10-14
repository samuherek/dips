use crate::configuration;
use crate::models::dip::{self, DipRowFull};
use crate::models::dir_context::{self, ContextScope};
use crate::tui;
use color_eyre::eyre::WrapErr;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures_util::stream::StreamExt;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::palette::tailwind::SLATE;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap,
};
use ratatui::Frame;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

#[derive(Debug, Default)]
enum EventCtx {
    #[default]
    List,
    Search,
}

#[derive(Debug, Default)]
enum View {
    #[default]
    ScopeList,
}

#[derive(Debug, Default, PartialEq)]
enum Mode {
    #[default]
    Running,
    Quit,
}

#[derive(Debug)]
struct AppState {
    mode: Mode,
    view: View,
    event_context: EventCtx,
    scope_dips: Vec<DipRowFull>,
    list_selection_index: Option<usize>,
    search: String,
    scope: ContextScope,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: Mode::default(),
            view: View::default(),
            scope_dips: Vec::default(),
            event_context: EventCtx::default(),
            search: String::default(),
            list_selection_index: None,
            scope: ContextScope::Global,
        }
    }

    fn is_running(&self) -> bool {
        self.mode == Mode::Running
    }
}

fn render_header(scope_path: &str, area: Rect, frame: &mut Frame) {
    let text = Line::from(vec![Span::raw("Scope: "), Span::raw(scope_path)]);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::new()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::new().fg(SLATE.c500)),
            )
            .wrap(Wrap { trim: true }),
        area,
    )
}

fn render_scope_list(
    items: &Vec<DipRowFull>,
    selected_index: Option<usize>,
    area: Rect,
    frame: &mut Frame,
) {
    let items = items
        .iter()
        .map(|x| ListItem::new(x.value.clone()))
        .collect::<Vec<_>>();
    let list = List::new(items)
        .block(Block::new())
        .highlight_style(Style::new().bg(SLATE.c800))
        .highlight_symbol("> ")
        .highlight_spacing(HighlightSpacing::Always);

    let mut state = ListState::default().with_selected(selected_index);
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_toolbar(area: Rect, frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("<Esc> to exit"),
            Span::raw("  |  "),
            Span::raw("< / > to search"),
        ]))
        .block(Block::new()),
        area,
    );
}

fn render_search(value: &str, area: Rect, frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::raw("Search: "), Span::from(value)]))
            .block(Block::new()),
        area,
    );
}

#[derive(Debug)]
struct DipsFilter {
    scope_id: Option<String>,
    search: Option<String>,
}

#[derive(Debug)]
enum DbQuery {
    Dips(DipsFilter),
}

#[derive(Debug)]
enum DbResult {
    Dips(Vec<DipRowFull>),
}

#[derive(Debug)]
enum Command {
    Search,
}

#[derive(Debug)]
enum Event {
    DbRequest(DbQuery),
    DbResponse(DbResult),
    KeyboardEsc,
    KeyboardCtrlC,
    KeyboardChar(char),
    KeyboardBackspace,
    Command(Command),
    NavDown,
    NavUp,
    UiTick,
}

struct EventService {
    crossterm_events: EventStream,
    events: mpsc::UnboundedReceiver<Event>,
}

impl EventService {
    fn new(events: mpsc::UnboundedReceiver<Event>) -> Self {
        Self {
            crossterm_events: EventStream::new(),
            events,
        }
    }

    fn handle_crossterm(&self, event: CrosstermEvent, ctx: &EventCtx) -> Option<Event> {
        match event {
            CrosstermEvent::Key(key_event) if key_event.kind == KeyEventKind::Press => match ctx {
                EventCtx::List => match (key_event.code, key_event.modifiers) {
                    (KeyCode::Esc, _) => Some(Event::KeyboardEsc),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Event::KeyboardCtrlC),
                    (KeyCode::Char('j'), _) => Some(Event::NavDown),
                    (KeyCode::Char('k'), _) => Some(Event::NavUp),
                    (KeyCode::Char('/'), _) => Some(Event::Command(Command::Search)),
                    (_, _) => None,
                },
                EventCtx::Search => match (key_event.code, key_event.modifiers) {
                    (KeyCode::Esc, _) => Some(Event::KeyboardEsc),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Event::KeyboardCtrlC),
                    (KeyCode::Backspace, _) => Some(Event::KeyboardBackspace),
                    (KeyCode::Char(c), _) => Some(Event::KeyboardChar(c)),
                    (_, _) => None,
                },
            },
            _ => None,
        }
    }

    async fn next(&mut self, ctx: &EventCtx) -> color_eyre::Result<Event> {
        loop {
            let ev = tokio::select! {
                event = self.events.recv() => event,
                event = self.crossterm_events.next() => match event {
                    Some(Ok(ev)) => self.handle_crossterm(ev, ctx),
                    Some(Err(_)) => None,
                    None => None
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => Some(Event::UiTick),
            };
            if let Some(ev) = ev {
                return Ok(ev);
            }
        }
    }
}

struct QueryManager {
    db_pool: SqlitePool,
    sender: mpsc::UnboundedSender<Event>,
}

impl QueryManager {
    fn new(db_pool: SqlitePool, sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { db_pool, sender }
    }

    fn dips(&self, filter: DipsFilter) {
        let pool = self.db_pool.clone();
        let sender = self.sender.clone();
        tokio::spawn(async move {
            // TODO:: use the global dips::query stuff
            let search = format!("%{}%", filter.search.unwrap_or_default());
            let res: Vec<DipRowFull> = sqlx::query_as(
                r"
       select dips.*, 
            dir_contexts.dir_path, 
            dir_contexts.git_remote, 
            dir_contexts.git_dir_name 
        from dips
        left join dir_contexts on dips.dir_context_id = dir_contexts.id
        WHERE dips.dir_context_id = $1
        and LOWER(dips.value) LIKE LOWER($2)
        ",
            )
            .bind(filter.scope_id)
            .bind(search)
            .fetch_all(&pool)
            .await
            .expect("Failed to query database");
            let _ = sender.send(Event::DbResponse(DbResult::Dips(res)));
        });
    }
}

pub async fn exec(config: configuration::Application) -> color_eyre::Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let mut app_state = AppState::new();
    let (tx, rx) = mpsc::unbounded_channel();
    let mut events = EventService::new(rx);
    let scope = dir_context::get_closest(&config.db_pool, &config.context_dir)
        .await
        .expect("Failed to get dir context");
    app_state.scope = scope;

    let query_mgr = QueryManager::new(config.db_pool, tx.clone());
    let _ = tx.send(Event::DbRequest(DbQuery::Dips(DipsFilter {
        scope_id: app_state.scope.id(),
        search: Some("".into()),
    })));

    while app_state.is_running() {
        terminal
            .draw(|frame| {
                let layout = Layout::new(
                    Direction::Vertical,
                    vec![
                        Constraint::Length(2),
                        Constraint::Min(2),
                        Constraint::Length(1),
                    ],
                );
                let [header, main, footer] = layout.areas(frame.size());
                render_header(&app_state.scope.label(), header, frame);
                match app_state.view {
                    View::ScopeList => render_scope_list(
                        &app_state.scope_dips,
                        app_state.list_selection_index.to_owned(),
                        main,
                        frame,
                    ),
                };
                match app_state.event_context {
                    EventCtx::List => render_toolbar(footer, frame),
                    EventCtx::Search => render_search(&app_state.search, footer, frame),
                }
            })
            .wrap_err("terminal.draw")?;

        match events.next(&app_state.event_context).await? {
            Event::KeyboardCtrlC => app_state.mode = Mode::Quit,
            Event::KeyboardEsc => match app_state.event_context {
                EventCtx::Search => {
                    app_state.event_context = EventCtx::List;
                    app_state.search.clear();
                    query_mgr.dips(DipsFilter {
                        scope_id: app_state.scope.id(),
                        search: None,
                    })
                }
                _ => app_state.mode = Mode::Quit,
            },
            Event::DbRequest(query) => match query {
                DbQuery::Dips(filter) => query_mgr.dips(filter),
            },
            Event::DbResponse(result) => match result {
                DbResult::Dips(items) => {
                    app_state.scope_dips = items;
                    app_state.list_selection_index = Some(0);
                }
            },
            Event::NavUp => {
                if let Some(idx) = app_state.list_selection_index {
                    if app_state.scope_dips.len() > 0 {
                        app_state.list_selection_index = Some(idx.saturating_sub(1));
                    }
                }
            }
            Event::NavDown => {
                if let Some(idx) = app_state.list_selection_index {
                    if app_state.scope_dips.len() > 0 {
                        app_state.list_selection_index =
                            Some(idx.saturating_add(1).min(app_state.scope_dips.len() - 1));
                    }
                }
            }
            Event::KeyboardChar(c) => match app_state.event_context {
                EventCtx::Search => {
                    app_state.search.push(c);
                    query_mgr.dips(DipsFilter {
                        scope_id: app_state.scope.id(),
                        search: Some(app_state.search.to_owned()),
                    })
                }
                _ => {}
            },
            Event::KeyboardBackspace => match app_state.event_context {
                EventCtx::Search => {
                    let _ = app_state.search.pop();
                    query_mgr.dips(DipsFilter {
                        scope_id: app_state.scope.id(),
                        search: Some(app_state.search.to_owned()),
                    })
                }
                _ => {}
            },
            Event::Command(cmd) => match cmd {
                Command::Search => {
                    app_state.event_context = EventCtx::Search;
                    app_state.search.clear();
                }
            },
            _ => {}
        }
    }

    tui::restore()?;
    Ok(())
}
