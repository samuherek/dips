use crate::configuration;
use crate::models::dip::{self, DipRowFull, DipsFilter};
use crate::models::dir_context::{self, ContextScope, ScopesFilter};
use crate::models::tag;
use crate::tui;
use color_eyre::eyre::WrapErr;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures_util::stream::StreamExt;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::palette::tailwind::{GRAY, RED, SLATE, YELLOW};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph, Widget,
};
use ratatui::Frame;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Default, PartialEq)]
enum EventCtx {
    #[default]
    List,
    Search,
    SearchList,
    Tag,
    Confirm(Confirmation),
}

#[derive(Debug, PartialEq)]
enum Confirmation {
    Delete,
}

#[derive(Debug, Default)]
enum View {
    #[default]
    ScopeList,
    ScopeChange,
}

#[derive(Debug, Default, PartialEq)]
enum Mode {
    #[default]
    Running,
    Quit,
}

#[derive(Debug)]
enum PromptStyle {
    Normal,
    Info,
    Danger,
}

#[derive(Debug)]
enum PromptMode {
    Help,
    Nav,
    Input,
    Search,
    Confirm,
    Message,
}

#[derive(Debug)]
struct PromptState {
    input: String,
    msg: Option<&'static str>,
    style: PromptStyle,
    mode: PromptMode,
}

impl PromptState {
    fn reset(&mut self) {
        self.msg = None;
        self.style = PromptStyle::Normal;
        self.input.clear();
    }
}

impl Default for PromptState {
    fn default() -> Self {
        Self {
            input: String::default(),
            msg: None,
            style: PromptStyle::Normal,
            mode: PromptMode::Help,
        }
    }
}

#[derive(Debug, Clone)]
enum PageType {
    Dips,
    Scope,
    Help,
    Splash,
}

impl PageType {
    fn from_page(page: &PageState) -> Self {
        match page {
            PageState::Dips { .. } => Self::Dips,
            PageState::Scope { .. } => Self::Scope,
            PageState::Help => Self::Help,
            PageState::Splash => Self::Splash,
        }
    }
}

#[derive(Debug)]
enum PageState {
    Splash,
    Dips {
        scope_id: String,
        index: usize,
        items: Vec<uuid::Uuid>,
    },
    Scope {
        index: usize,
    },
    Help,
}

impl PageState {
    fn layout_with_prompt(&self) -> bool {
        true
    }

    fn page_type(&self) -> PageType {
        PageType::from_page(self)
    }

    fn action_move_up(&mut self) {
        match self {
            Self::Dips { index, items, .. } => {
                if !items.is_empty() {
                    *index = index.saturating_sub(1);
                }
            }
            _ => {}
        }
    }

    fn action_move_down(&mut self) {
        match self {
            Self::Dips { index, items, .. } => {
                if !items.is_empty() {
                    *index = index.saturating_add(1).min(items.len() - 1);
                }
            }
            _ => {}
        }
    }
}

impl Default for PageState {
    fn default() -> Self {
        Self::Splash
    }
}

#[derive(Debug)]
enum EventFocusMode {
    Page,
    Prompt,
}

#[derive(Debug)]
struct UiState {
    page: PageState,
    prompt: PromptState,
    event_focus: EventFocusMode,
    back_page: Option<PageType>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            page: PageState::default(),
            prompt: PromptState::default(),
            event_focus: EventFocusMode::Page,
            back_page: None,
        }
    }
}

impl UiState {
    fn from_type(page: &PageType, state: &AppState) -> PageState {
        match page {
            PageType::Dips => {
                let scope_id = state.scope.id().expect("Failed to get scope id");
                let items = state
                    .data
                    .dips
                    .iter()
                    .map(|(id, _)| id.to_owned())
                    .collect();
                PageState::Dips {
                    scope_id,
                    index: 0,
                    items,
                }
            }
            PageType::Help => PageState::Help,
            _ => todo!(),
        }
    }

    fn navigate(page: &PageType, state: &mut AppState) {
        state.ui.back_page = Some(state.ui.page.page_type());
        state.ui.event_focus = EventFocusMode::Page;
        state.ui.page = UiState::from_type(page, state);
        // TODO: this is the case only if it's help page for now.
        state.ui.prompt.mode = PromptMode::Nav;
    }

    fn navigate_back(state: &mut AppState) {
        let back_page = state
            .ui
            .back_page
            .as_ref()
            .expect("Failed to get back page");
        state.ui.page = UiState::from_type(back_page, state);
        state.ui.back_page = None;
        state.ui.prompt.mode = PromptMode::Help;
    }
}

#[derive(Debug)]
struct DataState {
    dips: HashMap<uuid::Uuid, DipRowFull>,
}

impl Default for DataState {
    fn default() -> Self {
        Self {
            dips: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct AppState {
    mode: Mode,
    view: View,

    ui: UiState,
    data: DataState,

    event_context: EventCtx,
    scope_dips: Vec<DipRowFull>,
    scope_items: Vec<ContextScope>,
    list_selection_index: Option<usize>,
    search: String,
    scope: ContextScope,
    error: Option<&'static str>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: Mode::default(),
            view: View::default(),
            ui: UiState::default(),
            data: DataState::default(),
            scope_dips: Vec::default(),
            scope_items: Vec::default(),
            event_context: EventCtx::default(),
            search: String::default(),
            list_selection_index: None,
            scope: ContextScope::Global,
            error: None,
        }
    }

    fn is_running(&self) -> bool {
        self.mode == Mode::Running
    }
}

fn render_dips_page(
    scope: &ContextScope,
    items: Vec<&DipRowFull>,
    index: usize,
    area: Rect,
    frame: &mut Frame,
) {
    let page_layout = Layout::new(
        Direction::Vertical,
        [Constraint::Length(2), Constraint::Min(0)],
    );
    let [header, main] = page_layout.areas(area);
    frame.render_widget(
        Paragraph::new(Text::from(format!("Scope: {}", scope.label()))),
        header,
    );

    let index = if items.len() > 0 { Some(index) } else { None };

    let items = items
        .iter()
        .map(|x| {
            ListItem::new(Line::from(vec![
                Span::raw(x.value.as_str()),
                Span::raw(" "),
                Span::from(format!("{}", x.tags.to_string())).style(Style::new().fg(SLATE.c500)),
            ]))
        })
        .collect::<Vec<_>>();
    let list = List::new(items)
        .block(Block::new())
        .highlight_style(Style::new().bg(SLATE.c800))
        .highlight_symbol("> ")
        .highlight_spacing(HighlightSpacing::Always);

    let mut state = ListState::default().with_selected(index);
    frame.render_stateful_widget(list, main, &mut state);
}

fn render_prompt(prompt: &PromptState, area: Rect, frame: &mut Frame) {
    match prompt.mode {
        PromptMode::Help => {
            let layout = Layout::new(Direction::Horizontal, Constraint::from_fills([1, 1]));
            let [left, right] = layout.areas(area);
            let left_widget =
                Line::from("Type : to start a command").style(Style::new().fg(GRAY.c500));
            let right_widget = Line::from(vec![
                Span::raw("   Search "),
                Span::styled(" / ", Style::new().bg(SLATE.c800).fg(GRAY.c400)),
                Span::raw("   Help "),
                Span::styled(" ? ", Style::new().bg(SLATE.c800).fg(GRAY.c400)),
                Span::raw("   Exit "),
                Span::styled(" C-c ", Style::new().bg(SLATE.c800).fg(GRAY.c400)),
            ])
            .style(Style::new().fg(GRAY.c200))
            .alignment(Alignment::Right);

            frame.render_widget(left_widget, left);
            frame.render_widget(right_widget, right);
        }
        PromptMode::Nav => {
            let line = Line::from(vec![
                Span::raw(" Go back "),
                Span::styled(" Esc ", Style::new().bg(SLATE.c800).fg(GRAY.c400)),
            ])
            .style(Style::new().fg(GRAY.c200))
            .alignment(Alignment::Left);
            frame.render_widget(line, area);
        }
        PromptMode::Input => {
            let layout = Layout::new(
                Direction::Horizontal,
                [Constraint::Min(0), Constraint::Length(20)],
            );
            let [left, right] = layout.areas(area);
            let left_widget = Line::from(vec![Span::raw("Command: "), Span::from(&prompt.input)])
                .style(Style::new().bg(SLATE.c800));
            let right_widget = Line::from(vec![
                Span::styled("To cancel ", Style::new().fg(GRAY.c500)),
                Span::styled(" Esc ", Style::new().bg(SLATE.c600).fg(GRAY.c400)),
            ])
            .style(Style::new().fg(GRAY.c600).bg(SLATE.c800))
            .alignment(Alignment::Right);
            frame.render_widget(left_widget, left);
            frame.render_widget(right_widget, right);
        }
        PromptMode::Search => {
            let layout = Layout::new(
                Direction::Horizontal,
                [Constraint::Min(0), Constraint::Length(20)],
            );
            let [left, right] = layout.areas(area);
            let left_widget = Line::from(vec![Span::raw("Search: "), Span::from(&prompt.input)])
                .style(Style::new().bg(SLATE.c800));
            let right_widget = Line::from(vec![
                Span::styled("To cancel ", Style::new().fg(GRAY.c500)),
                Span::styled(" Esc ", Style::new().bg(SLATE.c600).fg(GRAY.c400)),
            ])
            .style(Style::new().fg(GRAY.c600).bg(SLATE.c800))
            .alignment(Alignment::Right);
            frame.render_widget(left_widget, left);
            frame.render_widget(right_widget, right);
        }
        PromptMode::Confirm => {
            todo!()
        }
        PromptMode::Message => {
            todo!()
        }
    };
}

fn render_help_page(area: Rect, frame: &mut Frame) {
    let text = Paragraph::new(Line::from(
        "This is some awesome help text I need to figure out.",
    ));
    frame.render_widget(text, area);
}

fn render_page_with_prompt(state: &AppState, frame: &mut Frame) {
    let layout = Layout::new(
        Direction::Vertical,
        vec![Constraint::Min(2), Constraint::Length(1)],
    );
    let [page, prompt] = layout.areas(frame.size());
    match &state.ui.page {
        PageState::Dips { index, .. } => {
            let items = state
                .data
                .dips
                .iter()
                .map(|(_, val)| val)
                .collect::<Vec<_>>();
            let scope = &state.scope;
            render_dips_page(scope, items, *index, page, frame);
        }
        PageState::Help => {
            render_help_page(page, frame);
        }
        _ => {}
    };
    render_prompt(&state.ui.prompt, prompt, frame);
}

#[derive(Debug)]
enum DbQuery {
    Dips(DipsFilter),
    Scopes(ScopesFilter),
}

#[derive(Debug)]
enum DbResult {
    Dips(Vec<DipRowFull>),
    Scopes(Vec<ContextScope>),
    Tag,
    Remove,
}

#[derive(Debug)]
enum Command {
    Search,
    Tag,
    Confirm(Confirmation),
}

#[derive(Debug)]
enum PromptAction {
    Focus,
    Defocus,
    SearchInit,
    Input(char),
    InputBackspace,
}

#[derive(Debug)]
enum Action {
    MoveUp,
    MoveDown,
}

#[derive(Debug)]
enum Event {
    DbRequest(DbQuery),
    DbResponse(DbResult),
    KeyboardEsc,
    KeyboardChar(char),
    KeyboardBackspace,
    KeyboardEnter,
    Command(Command),
    NavDown,
    NavUp,
    ChangeScope,
    UiTick,
    Error(&'static str),

    Action(Action),
    Prompt(PromptAction),
    Nav(PageType),
    NavBack,
    QuitSignal,
}

struct EventService {
    crossterm_events: EventStream,
    events: mpsc::UnboundedReceiver<Event>,
    dispatcher: mpsc::UnboundedSender<Event>,
}

impl EventService {
    fn new(
        events: mpsc::UnboundedReceiver<Event>,
        dispatcher: mpsc::UnboundedSender<Event>,
    ) -> Self {
        Self {
            crossterm_events: EventStream::new(),
            events,
            dispatcher,
        }
    }

    fn handle_global_events(event: &KeyEvent) -> Option<Event> {
        match (event.code, event.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Event::QuitSignal),
            _ => None,
        }
    }

    fn handle_help_events(event: &KeyEvent, _ctx: &AppState) -> Option<Event> {
        match event.code {
            KeyCode::Esc => Some(Event::NavBack),
            _ => None,
        }
    }

    fn handle_page_events(event: &KeyEvent, _ctx: &AppState) -> Option<Event> {
        match event.code {
            KeyCode::Char('?') => Some(Event::Nav(PageType::Help)),
            KeyCode::Char('j') | KeyCode::Down => Some(Event::Action(Action::MoveDown)),
            KeyCode::Char('k') | KeyCode::Up => Some(Event::Action(Action::MoveUp)),
            KeyCode::Char(':') => Some(Event::Prompt(PromptAction::Focus)),
            KeyCode::Char('/') => Some(Event::Prompt(PromptAction::SearchInit)),
            _ => None,
        }
    }

    fn handle_prompt_events(event: &KeyEvent, _ctx: &AppState) -> Option<Event> {
        match event.code {
            KeyCode::Esc => Some(Event::Prompt(PromptAction::Defocus)),
            KeyCode::Backspace => Some(Event::Prompt(PromptAction::InputBackspace)),
            KeyCode::Char(c) => Some(Event::Prompt(PromptAction::Input(c))),
            _ => None,
        }
    }

    fn handle_key_events(&self, event: KeyEvent, ctx: &AppState) -> Option<Event> {
        if let Some(ev) = Self::handle_global_events(&event) {
            return Some(ev);
        }

        match ctx.ui.event_focus {
            EventFocusMode::Page => match ctx.ui.page {
                PageState::Help => Self::handle_help_events(&event, ctx),
                _ => Self::handle_page_events(&event, ctx),
            },
            EventFocusMode::Prompt => Self::handle_prompt_events(&event, ctx),
        }

        // match ctx.event_context {
        //     EventCtx::List => match (event.code, event.modifiers) {
        //         (KeyCode::Esc, _) => Some(Event::KeyboardEsc),
        //         (KeyCode::Char('p'), KeyModifiers::CONTROL) => Some(Event::ChangeScope),
        //         (KeyCode::Char('j'), _) => Some(Event::NavDown),
        //         (KeyCode::Char('k'), _) => Some(Event::NavUp),
        //         (KeyCode::Char('/'), _) => Some(Event::Command(Command::Search)),
        //         (KeyCode::Char('t'), _) => Some(Event::Command(Command::Tag)),
        //         (KeyCode::Char('d'), _) => {
        //             Some(Event::Command(Command::Confirm(Confirmation::Delete)))
        //         }
        //         (KeyCode::Enter, _) => Some(Event::KeyboardEnter),
        //         (_, _) => None,
        //     },
        //     EventCtx::Search => match (event.code, event.modifiers) {
        //         (KeyCode::Esc, _) => Some(Event::KeyboardEsc),
        //         (KeyCode::Backspace, _) => Some(Event::KeyboardBackspace),
        //         (KeyCode::Char(c), _) => Some(Event::KeyboardChar(c)),
        //         (KeyCode::Enter, _) => Some(Event::KeyboardEnter),
        //         (_, _) => None,
        //     },
        //     EventCtx::SearchList => match (event.code, event.modifiers) {
        //         (KeyCode::Esc, _) => Some(Event::KeyboardEsc),
        //         (KeyCode::Char('j'), _) => Some(Event::NavDown),
        //         (KeyCode::Char('k'), _) => Some(Event::NavUp),
        //         (KeyCode::Char('/'), _) => Some(Event::Command(Command::Search)),
        //         (KeyCode::Char('d'), _) => {
        //             Some(Event::Command(Command::Confirm(Confirmation::Delete)))
        //         }
        //         (_, _) => None,
        //     },
        //     EventCtx::Tag => match (event.code, event.modifiers) {
        //         (KeyCode::Esc, _) => Some(Event::KeyboardEsc),
        //         (KeyCode::Backspace, _) => Some(Event::KeyboardBackspace),
        //         (KeyCode::Char(c), _) => Some(Event::KeyboardChar(c)),
        //         (KeyCode::Enter, _) => Some(Event::KeyboardEnter),
        //         (_, _) => None,
        //     },
        //     EventCtx::Confirm(_) => match (event.code, event.modifiers) {
        //         (KeyCode::Esc, _) => Some(Event::KeyboardEsc),
        //         (KeyCode::Char(c), _) => Some(Event::KeyboardChar(c)),
        //         (KeyCode::Backspace, _) => Some(Event::KeyboardBackspace),
        //         (KeyCode::Enter, _) => Some(Event::KeyboardEnter),
        //         (_, _) => None,
        //     },
        // }
    }

    fn send(&self, event: Event) {
        if self.dispatcher.send(event).is_err() {
            eprintln!("Failed to dispatch an event");
        }
    }

    async fn next(&mut self, ctx: &AppState) -> color_eyre::Result<Event> {
        loop {
            let ev = tokio::select! {
                event = self.events.recv() => event,
                event = self.crossterm_events.next() => match event {
                    Some(Ok(ev)) => {
                    match ev {
                        CrosstermEvent::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                           self.handle_key_events(key_event, ctx)
                        },
                        _ => None
                    }
                    }
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

    fn dips(&self, state: &AppState) {
        let filter = DipsFilter::new()
            .with_scope_id(state.scope.id())
            .with_search(&state.search);
        let pool = self.db_pool.clone();
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let res = dip::get_filtered(&pool, filter)
                .await
                .expect("Failed to query filtered dips");
            let _ = sender.send(Event::DbResponse(DbResult::Dips(res)));
        });
    }

    fn tag_dip(&self, state: &AppState) {
        let item = state
            .list_selection_index
            .and_then(|x| state.scope_dips.get(x));
        match item {
            Some(item) => {
                let tag = state.search.to_owned();
                let id = item.id.to_owned();
                let pool = self.db_pool.clone();
                let sender = self.sender.clone();
                tokio::spawn(async move {
                    let mut tx = pool.begin().await.expect("Failed to create transaction");
                    tag::create_dip_tag(&mut tx, &id, &tag)
                        .await
                        .expect("Failed to create a tag for a dip");
                    tx.commit().await.expect("Failed to commit a tag for a dip");
                    let _ = sender.send(Event::DbResponse(DbResult::Tag));
                });
            }
            None => {
                let _ = self
                    .sender
                    .send(Event::Error("Could not find item to tag."));
            }
        }
    }

    fn remove_dip(&self, state: &AppState) {
        let item = state
            .list_selection_index
            .and_then(|x| state.scope_dips.get(x));
        match item {
            Some(item) => {
                let id = item.id.to_owned();
                let pool = self.db_pool.clone();
                let sender = self.sender.clone();
                tokio::spawn(async move {
                    dip::delete(&pool, &id.to_string())
                        .await
                        .expect("Failed to delete a dip");
                    let _ = sender.send(Event::DbResponse(DbResult::Remove));
                });
            }
            None => {
                let _ = self
                    .sender
                    .send(Event::Error("Could not find item to tag."));
            }
        }
    }
    fn scopes(&self, state: &AppState) {
        let pool = self.db_pool.clone();
        let sender = self.sender.clone();
        let filter = ScopesFilter::new().with_search(&state.search);
        tokio::spawn(async move {
            let res = dir_context::get_filtered(&pool, filter)
                .await
                .expect("Failed to query filtered scopes");
            let _ = sender.send(Event::DbResponse(DbResult::Scopes(res)));
        });
    }
}

pub async fn exec(config: configuration::Application) -> color_eyre::Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let mut app_state = AppState::new();
    let (tx, rx) = mpsc::unbounded_channel();
    let mut events = EventService::new(rx, tx.clone());
    let scope = dir_context::get_closest(&config.db_pool, &config.context_dir)
        .await
        .expect("Failed to get dir context");
    app_state.scope = scope;

    let query_mgr = QueryManager::new(config.db_pool, tx.clone());

    events.send(Event::DbRequest(DbQuery::Dips(
        DipsFilter::new()
            .with_scope_id(app_state.scope.id())
            .with_search(""),
    )));

    while app_state.is_running() {
        terminal
            .draw(|frame| {
                if app_state.ui.page.layout_with_prompt() {
                    render_page_with_prompt(&app_state, frame);
                }

                // let layout = Layout::new(
                //     Direction::Vertical,
                //     vec![
                //         Constraint::Length(2),
                //         Constraint::Min(2),
                //         Constraint::Length(1),
                //     ],
                // );
                // let [page, prompt] = layout.areas(frame.size());
                // match app_state.view {
                //     View::ScopeList => {
                //         render_scope_header(&app_state.scope.label(), header, frame);
                //         render_scope_list(
                //             &app_state.scope_dips,
                //             app_state.list_selection_index.to_owned(),
                //             main,
                //             frame,
                //         );
                //     }
                //     View::ScopeChange => {
                //         render_context_header(header, frame);
                //         render_context_list(
                //             &app_state.scope_items,
                //             app_state.list_selection_index.to_owned(),
                //             main,
                //             frame,
                //         );
                //     }
                // };
                // if let Some(err) = app_state.error {
                //     render_error(err, footer, frame);
                // } else {
                //     match &app_state.event_context {
                //         EventCtx::List => render_toolbar(footer, frame),
                //         EventCtx::Search => render_search(&app_state.search, footer, frame),
                //         EventCtx::SearchList => {
                //             render_search_list(&app_state.search, footer, frame)
                //         }
                //         EventCtx::Tag => render_tag(&app_state.search, footer, frame),
                //         EventCtx::Confirm(kind) => {
                //             render_confirm(kind, &app_state.search, footer, frame)
                //         }
                //     }
                // }
            })
            .wrap_err("terminal.draw")?;

        match events.next(&app_state).await? {
            Event::QuitSignal => app_state.mode = Mode::Quit,
            Event::KeyboardEsc => match app_state.event_context {
                EventCtx::Search | EventCtx::SearchList => {
                    app_state.event_context = EventCtx::List;
                    app_state.search.clear();
                    query_mgr.dips(&app_state)
                }
                EventCtx::Tag => {
                    app_state.event_context = EventCtx::List;
                    app_state.search.clear();
                }
                EventCtx::Confirm(_) => app_state.event_context = EventCtx::List,
                EventCtx::List => app_state.mode = Mode::Quit,
            },
            Event::DbRequest(query) => match query {
                DbQuery::Dips(_) => query_mgr.dips(&app_state),
                DbQuery::Scopes(_) => query_mgr.scopes(&app_state),
            },
            Event::DbResponse(result) => match result {
                DbResult::Dips(items) => {
                    app_state.data.dips = items.into_iter().map(|x| (x.id.to_owned(), x)).collect();
                    app_state.ui.page = UiState::from_type(&PageType::Dips, &app_state);
                }
                DbResult::Tag | DbResult::Remove => {
                    query_mgr.dips(&app_state);
                }
                DbResult::Scopes(items) => {
                    app_state.scope_items = items;
                    app_state.list_selection_index = Some(0);
                }
            },
            Event::ChangeScope => {
                app_state.view = View::ScopeChange;
                query_mgr.scopes(&app_state);
            }
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
                    query_mgr.dips(&app_state)
                }
                EventCtx::Tag | EventCtx::Confirm(_) => {
                    app_state.search.push(c);
                }
                EventCtx::List | EventCtx::SearchList => {}
            },
            Event::KeyboardBackspace => match app_state.event_context {
                EventCtx::Search => {
                    let _ = app_state.search.pop();
                    query_mgr.dips(&app_state);
                }
                EventCtx::Tag | EventCtx::Confirm(_) => {
                    let _ = app_state.search.pop();
                }
                EventCtx::List | EventCtx::SearchList => {}
            },
            Event::KeyboardEnter => match app_state.event_context {
                EventCtx::Search => {
                    app_state.event_context = EventCtx::SearchList;
                }
                EventCtx::SearchList => match app_state.view {
                    View::ScopeChange => {
                        let item = app_state
                            .list_selection_index
                            .and_then(|x| app_state.scope_items.get(x));
                        if let Some(item) = item {
                            app_state.scope = item.clone();
                            app_state.view = View::ScopeList;
                            app_state.search.clear();
                            app_state.event_context = EventCtx::List;
                            query_mgr.dips(&app_state);
                        }
                    }
                    View::ScopeList => {}
                },
                EventCtx::List => match app_state.view {
                    View::ScopeChange => {
                        let item = app_state
                            .list_selection_index
                            .and_then(|x| app_state.scope_items.get(x));
                        if let Some(item) = item {
                            app_state.scope = item.clone();
                            app_state.view = View::ScopeList;
                            app_state.search.clear();
                            app_state.event_context = EventCtx::List;
                            query_mgr.dips(&app_state);
                        }
                    }
                    View::ScopeList => {}
                },
                EventCtx::Confirm(_) => {
                    match app_state.search.to_lowercase().as_str() {
                        "n" | "no" => {
                            app_state.search.clear();
                            app_state.event_context = EventCtx::List;
                        }
                        "y" | "yes" => {
                            query_mgr.remove_dip(&app_state);
                            app_state.search.clear();
                            app_state.event_context = EventCtx::List;
                        }
                        _ => {
                            todo!("Add a message that only yes or no is allowed.");
                        }
                    };
                }
                EventCtx::Tag => {
                    query_mgr.tag_dip(&app_state);
                    app_state.search.clear();
                    app_state.event_context = EventCtx::List;
                }
            },
            Event::Command(cmd) => {
                app_state.error = None;
                match cmd {
                    Command::Search => {
                        if app_state.event_context != EventCtx::SearchList {
                            app_state.search.clear();
                        }
                        app_state.event_context = EventCtx::Search;
                    }
                    Command::Tag => {
                        app_state.event_context = EventCtx::Tag;
                        app_state.search.clear();
                    }
                    Command::Confirm(value) => {
                        app_state.event_context = EventCtx::Confirm(value);
                    }
                }
            }
            Event::Error(msg) => {
                app_state.error = Some(msg);
                app_state.event_context = EventCtx::List;
            }
            Event::UiTick => {}
            Event::Action(action) => match action {
                Action::MoveUp => app_state.ui.page.action_move_up(),
                Action::MoveDown => app_state.ui.page.action_move_down(),
            },
            Event::Prompt(action) => match action {
                PromptAction::Focus => {
                    // TODO: Move out to some function
                    app_state.ui.event_focus = EventFocusMode::Prompt;
                    app_state.ui.prompt.reset();
                    app_state.ui.prompt.mode = PromptMode::Input;
                }
                PromptAction::Defocus => {
                    app_state.ui.event_focus = EventFocusMode::Page;
                    app_state.ui.prompt.mode = PromptMode::Help;
                }
                PromptAction::SearchInit => {
                    app_state.ui.event_focus = EventFocusMode::Prompt;
                    app_state.ui.prompt.reset();
                    app_state.ui.prompt.mode = PromptMode::Search;
                }
                PromptAction::Input(c) => {
                    app_state.ui.prompt.input.push(c);
                }
                PromptAction::InputBackspace => {
                    app_state.ui.prompt.input.pop();
                }
            },
            Event::Nav(page) => UiState::navigate(&page, &mut app_state),
            Event::NavBack => UiState::navigate_back(&mut app_state),
        }
    }

    tui::restore()?;
    Ok(())
}
