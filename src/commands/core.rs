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
use ratatui::style::palette::tailwind::{GRAY, SLATE};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, HighlightSpacing, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Default, PartialEq)]
enum Mode {
    #[default]
    Running,
    Quit,
}

#[derive(Debug, Default)]
enum PromptStyle {
    #[default]
    Normal,
    Info,
    Danger,
}

#[derive(Debug, Default)]
enum SearchState {
    #[default]
    Active,
    Commit,
}

#[derive(Debug, Default)]
enum PromptState {
    #[default]
    Help,
    Nav,
    Input {
        input: String,
    },
    Search {
        input: String,
        style: PromptStyle,
        state: SearchState,
    },
    Message {
        value: &'static str,
        style: PromptStyle,
    },
    Confirm,
}

impl PromptState {
    fn activate_input_state(&mut self) {
        *self = Self::Input {
            input: String::new(),
        }
    }

    fn activate_help_state(&mut self) {
        *self = Self::Help
    }

    fn activate_search_state(&mut self) {
        *self = Self::Search {
            input: Default::default(),
            style: Default::default(),
            state: Default::default(),
        }
    }

    fn activate_nav_state(&mut self) {
        *self = Self::Nav;
    }

    fn set_input(&mut self, c: char) -> bool {
        match self {
            Self::Search { input, .. } | Self::Input { input, .. } => {
                input.push(c);
                true
            }
            _ => false,
        }
    }

    fn set_input_backspace(&mut self) {
        match self {
            Self::Search { input, .. } | Self::Input { input, .. } => {
                input.pop();
            }
            _ => {}
        }
    }

    fn set_error(&mut self, value: &'static str) {
        *self = Self::Message {
            value,
            style: PromptStyle::Danger,
        }
    }

    fn set_submit(&mut self) {
        match self {
            Self::Search { state, .. } => {
                if let SearchState::Active = state {
                    *state = SearchState::Commit;
                }
            }
            _ => {
                self.set_error("Invalid submit state");
            }
        }
    }

    fn get_search_input(&self) -> &str {
        if let Self::Search { input, .. } = self {
            input.as_str()
        } else {
            ""
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PageType {
    Dips { scope_id: Uuid },
    Scope,
    Help,
    Splash,
}

impl PageType {
    fn from_page(page: &PageState) -> Self {
        match page {
            PageState::Dips { scope_id, .. } => Self::Dips {
                scope_id: scope_id.clone(),
            },
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
        scope_id: Uuid,
        index: usize,
        items: Vec<Uuid>,
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
    fn from_type(page: &PageType, data: &DataState) -> PageState {
        match page {
            PageType::Dips { scope_id } => {
                let items = data.dips.iter().map(|(id, _)| id.to_owned()).collect();
                PageState::Dips {
                    scope_id: *scope_id,
                    index: 0,
                    items,
                }
            }
            PageType::Help => PageState::Help,
            _ => todo!(),
        }
    }

    fn navigate(&mut self, page: &PageType, data: &DataState) {
        self.back_page = Some(self.page.page_type());
        self.event_focus = EventFocusMode::Page;
        self.page = UiState::from_type(page, data);
        // TODO: this is the case only if it's help page for now.
        if page == &PageType::Help {
            self.prompt.activate_nav_state();
        }
    }

    fn navigate_back(&mut self, page: &PageType, data: &DataState) {
        self.page = UiState::from_type(page, data);
        self.back_page = None;
        self.prompt.activate_help_state();
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

    ui: UiState,
    data: DataState,

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
            ui: UiState::default(),
            data: DataState::default(),
            scope_dips: Vec::default(),
            scope_items: Vec::default(),
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

fn dips_page_items<'a>(
    dips: &'a HashMap<uuid::Uuid, DipRowFull>,
    search: &str,
) -> Vec<&'a DipRowFull> {
    dips.iter()
        .map(|(_, val)| val)
        .filter(|x| x.value.contains(search))
        .collect::<Vec<_>>()
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
        [
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ],
    );
    let [header, border, main] = page_layout.areas(area);
    let scope_text = format!("  {}", scope.label());
    frame.render_widget(Paragraph::new(Line::from(scope_text)), header);
    frame.render_widget(
        Paragraph::new(Span::styled("  -------", Style::new().fg(GRAY.c500))),
        border,
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
    match prompt {
        PromptState::Help => {
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
        PromptState::Nav => {
            let line = Line::from(vec![
                Span::raw(" Go back "),
                Span::styled(" Esc ", Style::new().bg(SLATE.c800).fg(GRAY.c400)),
            ])
            .style(Style::new().fg(GRAY.c200))
            .alignment(Alignment::Left);
            frame.render_widget(line, area);
        }
        PromptState::Input { input } => {
            let layout = Layout::new(
                Direction::Horizontal,
                [Constraint::Min(0), Constraint::Length(20)],
            );
            let [left, right] = layout.areas(area);
            let left_widget = Line::from(vec![Span::raw("Command: "), Span::from(input)])
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
        PromptState::Search { input, .. } => {
            let layout = Layout::new(
                Direction::Horizontal,
                [Constraint::Min(0), Constraint::Length(20)],
            );
            let [left, right] = layout.areas(area);
            let left_widget = Line::from(vec![Span::raw("Search: "), Span::from(input)])
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
        PromptState::Confirm => {
            todo!()
        }
        PromptState::Message { .. } => {
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
            let items = dips_page_items(&state.data.dips, state.ui.prompt.get_search_input());
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
enum PromptAction {
    Focus,
    Defocus,
    SearchInit,
    Input(char),
    InputBackspace,
    Submit,
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
    UiTick,
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
            KeyCode::Enter => Some(Event::Prompt(PromptAction::Submit)),
            _ => None,
        }
    }

    fn handle_key_events(&self, event: KeyEvent, state: &AppState) -> Option<Event> {
        if let Some(ev) = Self::handle_global_events(&event) {
            return Some(ev);
        }

        match state.ui.event_focus {
            EventFocusMode::Page => match state.ui.page {
                PageState::Help => Self::handle_help_events(&event, state),
                _ => Self::handle_page_events(&event, state),
            },
            EventFocusMode::Prompt => Self::handle_prompt_events(&event, state),
        }
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
                todo!("Send error to the prompt");
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
                todo!("Send error to the prompt");
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

    // TODO: create a bootstrap query and setup
    events.send(Event::DbRequest(DbQuery::Dips(
        DipsFilter::new()
            .with_scope_id(app_state.scope.id())
            .with_search(""),
    )));
    // events.send(Event::Nav(PageType::Dips {
    //     scope_id: scope.id().expect("Have initial scope!!!"),
    // }));

    while app_state.is_running() {
        terminal
            .draw(|frame| {
                if app_state.ui.page.layout_with_prompt() {
                    render_page_with_prompt(&app_state, frame);
                }
            })
            .wrap_err("terminal.draw")?;

        match events.next(&app_state).await? {
            Event::QuitSignal => app_state.mode = Mode::Quit,
            Event::DbRequest(query) => match query {
                DbQuery::Dips(_) => query_mgr.dips(&app_state),
                DbQuery::Scopes(_) => query_mgr.scopes(&app_state),
            },
            Event::DbResponse(result) => match result {
                DbResult::Dips(items) => {
                    app_state.data.dips = items.into_iter().map(|x| (x.id.to_owned(), x)).collect();
                }
                DbResult::Tag | DbResult::Remove => {
                    query_mgr.dips(&app_state);
                }
                DbResult::Scopes(items) => {
                    app_state.scope_items = items;
                    app_state.list_selection_index = Some(0);
                }
            },
            Event::UiTick => {}
            Event::Action(action) => match action {
                Action::MoveUp => app_state.ui.page.action_move_up(),
                Action::MoveDown => app_state.ui.page.action_move_down(),
            },
            Event::Prompt(action) => match action {
                PromptAction::Focus => {
                    // TODO: Move out to some function
                    app_state.ui.event_focus = EventFocusMode::Prompt;
                    app_state.ui.prompt.activate_input_state();
                }
                PromptAction::Defocus => {
                    app_state.ui.event_focus = EventFocusMode::Page;
                    app_state.ui.prompt.activate_help_state();
                }
                PromptAction::SearchInit => {
                    app_state.ui.event_focus = EventFocusMode::Prompt;
                    app_state.ui.prompt.activate_search_state();
                }
                PromptAction::Input(c) => {
                    if !app_state.ui.prompt.set_input(c) {
                        app_state.ui.prompt.set_error("Can not type in this mode");
                    }
                }
                PromptAction::InputBackspace => {
                    app_state.ui.prompt.set_input_backspace();
                }
                PromptAction::Submit => {
                    app_state.ui.prompt.set_submit();
                }
            },
            Event::Nav(page) => app_state.ui.navigate(&page, &app_state.data),
            Event::NavBack => match app_state.ui.back_page {
                Some(ref page) => app_state.ui.navigate_back(&page.clone(), &app_state.data),
                None => todo!(),
            },
        }
    }

    tui::restore()?;
    Ok(())
}
