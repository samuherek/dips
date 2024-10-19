use crate::configuration;
use crate::models::dip::{self, DipRowFull, DipsFilter};
use crate::models::dir_context::{self, DirContext, ScopesFilter};
use crate::tui;
use color_eyre::eyre::WrapErr;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures_util::stream::StreamExt;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::palette::tailwind::{GRAY, RED, SLATE};
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
    Default,
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
    Confirm {
        input: String,
        command: Command,
    },
}

impl PromptState {
    fn activate_input_state(&mut self) {
        *self = Self::Input {
            input: String::new(),
        }
    }

    fn activate_default_state(&mut self) {
        *self = Self::Default
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

    fn activate_confirm_state(&mut self, command: Command) {
        *self = Self::Confirm {
            input: Default::default(),
            command,
        }
    }

    fn set_input(&mut self, c: char) -> bool {
        match self {
            Self::Search { input, .. }
            | Self::Input { input, .. }
            | Self::Confirm { input, .. } => {
                input.push(c);
                true
            }
            _ => false,
        }
    }

    fn set_input_backspace(&mut self) {
        match self {
            Self::Search { input, .. }
            | Self::Input { input, .. }
            | Self::Confirm { input, .. } => {
                input.pop();
            }
            _ => {}
        }
    }

    fn set_error(&mut self, value: &'static str) {
        self.handle_message(value, PromptStyle::Danger);
    }

    fn handle_commit(&mut self, dispatch: &mpsc::UnboundedSender<Event>) {
        match self {
            Self::Search { state, .. } => {
                if let SearchState::Active = state {
                    *state = SearchState::Commit;
                }
            }
            Self::Input { ref input } => {
                if let Some((cmd, rest)) = input.clone().split_once(" ") {
                    match cmd {
                        "add" => {
                            let _ = dispatch.send(Event::Command(Command::Add(rest.to_owned())));
                        }
                        _ => self.set_error("Unknonw command"),
                    }
                } else {
                    self.set_error("Invalid command pattern");
                }
            }
            Self::Confirm { input, command } => match command {
                Command::DeleteDip(id) => {
                    if input == "y" {
                        let _ = dispatch.send(Event::Command(Command::DeleteDip(*id)));
                    } else {
                        self.set_error("Only y is allowed");
                    }
                }
                _ => todo!(),
            },
            _ => {
                self.set_error("Invalid submit state");
            }
        }
    }

    fn handle_message(&mut self, value: &'static str, style: PromptStyle) {
        *self = Self::Message { value, style }
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
    Dips { scope_id: Option<Uuid> },
    Scopes,
    Help,
    Splash,
}

impl PageType {
    fn from_page(page: &PageState) -> Self {
        match page {
            PageState::Dips { scope_id, .. } => Self::Dips {
                scope_id: scope_id.clone(),
            },
            PageState::Scopes { .. } => Self::Scopes,
            PageState::Help => Self::Help,
            PageState::Splash => Self::Splash,
        }
    }
}

#[derive(Debug, Default)]
enum DipsFocus {
    #[default]
    List,
    Scope,
}

#[derive(Debug, Default)]
enum ScopesFocus {
    #[default]
    Global,
    List,
}

#[derive(Debug)]
enum PageState {
    Splash,
    Dips {
        scope_id: Option<Uuid>,
        index: usize,
        items: Vec<Uuid>,
        focus: DipsFocus,
    },
    Scopes {
        index: usize,
        items: Vec<Uuid>,
        focus: ScopesFocus,
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
            Self::Dips {
                index,
                items,
                focus,
                ..
            } => {
                if matches!(focus, DipsFocus::List) {
                    if !items.is_empty() && *index > 0 {
                        *index = index.saturating_sub(1);
                    } else {
                        *focus = DipsFocus::Scope;
                    }
                }
            }
            Self::Scopes {
                index,
                items,
                focus,
            } => {
                if matches!(focus, ScopesFocus::List) {
                    if !items.is_empty() && *index > 0 {
                        *index = index.saturating_sub(1);
                    } else {
                        *focus = ScopesFocus::Global;
                    }
                }
            }
            _ => {}
        }
    }

    fn action_move_down(&mut self) {
        match self {
            Self::Dips {
                index,
                items,
                focus,
                ..
            } => match focus {
                DipsFocus::List => {
                    if !items.is_empty() {
                        *index = index.saturating_add(1).min(items.len() - 1);
                    }
                }
                DipsFocus::Scope => {
                    *focus = DipsFocus::List;
                    *index = 0;
                }
            },
            Self::Scopes {
                index,
                items,
                focus,
            } => match focus {
                ScopesFocus::List => {
                    if !items.is_empty() {
                        *index = index.saturating_add(1).min(items.len() - 1);
                    }
                }
                ScopesFocus::Global => {
                    *focus = ScopesFocus::List;
                    *index = 0;
                }
            },
            _ => {}
        }
    }

    fn fetch_data(&self, qm: &QueryManager) {
        match self {
            PageState::Dips { scope_id, .. } => {
                let pool = qm.db_pool.clone();
                let sender = qm.sender.clone();
                let filter = DipsFilter::new().with_scope_id(scope_id.clone());
                tokio::spawn(async move {
                    let res = dip::get_filtered(&pool, filter)
                        .await
                        .expect("Failed to query filtered dips");
                    if sender
                        .send(Event::LoadData(DataPayload::Dips(res)))
                        .is_err()
                    {
                        todo!("report an error about the dispatch");
                    }
                });
            }
            PageState::Scopes { .. } => {
                let pool = qm.db_pool.clone();
                let sender = qm.sender.clone();
                let filter = ScopesFilter::new();
                tokio::spawn(async move {
                    let res = dir_context::get_filtered(&pool, filter)
                        .await
                        .expect("Failed to query filtered scopes");
                    if sender
                        .send(Event::LoadData(DataPayload::Scopes(res)))
                        .is_err()
                    {
                        todo!("report an error about the dispatch");
                    }
                });
            }
            PageState::Splash => {}
            PageState::Help => {}
        };
    }
}
fn handle_add_command(state: &mut UiState, qm: &QueryManager, value: String) {
    match state.page {
        PageState::Dips { scope_id, .. } => {
            let pool = qm.db_pool.clone();
            let sender = qm.sender.clone();
            let _ = sender.send(Event::Prompt(PromptEvent::Defocus));
            tokio::spawn(async move {
                let scope_id = scope_id.clone();
                match dip::create(&pool, scope_id.clone(), &value, None).await {
                    Ok(_) => sender.send(Event::RefetchData(PageType::Dips { scope_id })),
                    Err(_) => sender.send(Event::Prompt(PromptEvent::Message {
                        msg: "Failed to add the dip",
                        style: PromptStyle::Danger,
                    })),
                }
            });
        }
        _ => {
            state
                .prompt
                .set_error("\"add\" command not supported fro this view");
        }
    }
}

fn handle_delete_dip_command(state: &mut UiState, qm: &QueryManager, id: Uuid) {
    let pool = qm.db_pool.clone();
    let sender = qm.sender.clone();
    let scope_id = match state.page {
        PageState::Dips { scope_id, .. } => scope_id.clone(),
        _ => None,
    };
    let _ = sender.send(Event::Prompt(PromptEvent::Defocus));
    tokio::spawn(async move {
        match dip::delete(&pool, &id).await {
            Ok(_) => {
                let _ = sender.send(Event::Prompt(PromptEvent::Message {
                    msg: "Dip deleted",
                    style: PromptStyle::Info,
                }));
                let _ = sender.send(Event::RefetchData(PageType::Dips { scope_id }));
            }
            Err(_) => {
                let _ = sender.send(Event::Prompt(PromptEvent::Message {
                    msg: "Failed to delte the dip",
                    style: PromptStyle::Danger,
                }));
            }
        }
    });
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
    fn from_type(page: &PageType) -> PageState {
        match page {
            PageType::Dips { scope_id } => PageState::Dips {
                scope_id: *scope_id,
                index: 0,
                items: vec![],
                focus: DipsFocus::default(),
            },
            PageType::Help => PageState::Help,
            PageType::Scopes => PageState::Scopes {
                index: 0,
                items: vec![],
                focus: ScopesFocus::default(),
            },
            PageType::Splash => {
                unreachable!();
            }
        }
    }

    fn navigate(&mut self, page: &PageType) {
        self.back_page = Some(self.page.page_type());
        self.event_focus = EventFocusMode::Page;
        self.page = UiState::from_type(page);
        // TODO: this is the case only if it's help page for now.
        if page == &PageType::Help {
            self.prompt.activate_nav_state();
        }
    }

    fn navigate_back(&mut self, page: &PageType) {
        self.page = UiState::from_type(page);
        self.back_page = None;
        self.prompt.activate_default_state();
    }
}

#[derive(Debug)]
struct DataState {
    dips: HashMap<Uuid, DipRowFull>,
    scopes: HashMap<Uuid, DirContext>,
}

impl Default for DataState {
    fn default() -> Self {
        Self {
            dips: HashMap::new(),
            scopes: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct AppState {
    mode: Mode,
    ui: UiState,
    data: DataState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: Mode::default(),
            ui: UiState::default(),
            data: DataState::default(),
        }
    }

    fn is_running(&self) -> bool {
        self.mode == Mode::Running
    }

    fn load_dips_page(&mut self, items: Vec<DipRowFull>) {
        self.data.dips = items.into_iter().map(|x| (x.id.to_owned(), x)).collect();
        match self.ui.page {
            PageState::Dips {
                ref mut items,
                ref mut index,
                ..
            } => {
                *items = self.data.dips.iter().map(|(id, _)| id.clone()).collect();
                *index = 0;
            }
            _ => unreachable!(),
        };
    }

    fn load_scopes_page(&mut self, items: Vec<DirContext>) {
        self.data.scopes = items.into_iter().map(|x| (x.id.to_owned(), x)).collect();
        match self.ui.page {
            PageState::Scopes {
                ref mut items,
                ref mut index,
                ..
            } => {
                *items = self.data.scopes.iter().map(|(id, _)| id.clone()).collect();
                *index = 0;
            }
            _ => {}
        };
    }
}

fn render_dips_page(
    scope: Option<&DirContext>,
    items: Vec<&DipRowFull>,
    index: usize,
    focus: &DipsFocus,
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
    let mut scope_text = vec![];
    scope_text.push(Span::from(
        scope.map(|x| x.dir_path.as_str()).unwrap_or("Global"),
    ));

    if let Some(scope) = scope {
        scope_text.push(Span::raw(" "));
        scope_text.push(Span::styled(
            scope.git_remote.as_ref().map(|x| x.as_str()).unwrap_or(""),
            Style::new().fg(SLATE.c500),
        ));
    }

    let scope_style = match focus {
        DipsFocus::Scope => Style::new().bg(SLATE.c800),
        DipsFocus::List => Style::new(),
    };

    frame.render_widget(
        Paragraph::new(Line::from(scope_text)).style(scope_style),
        header,
    );
    frame.render_widget(
        Paragraph::new(Span::styled("-------", Style::new().fg(GRAY.c500))),
        border,
    );

    let index = if items.len() > 0 && matches!(focus, DipsFocus::List) {
        Some(index)
    } else {
        None
    };

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
        .highlight_spacing(HighlightSpacing::Never);

    let mut state = ListState::default().with_selected(index);
    frame.render_stateful_widget(list, main, &mut state);
}

fn render_prompt(prompt: &PromptState, area: Rect, frame: &mut Frame) {
    match prompt {
        PromptState::Default => {
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
        PromptState::Confirm { input, command } => match command {
            Command::DeleteDip(_) => {
                let layout = Layout::new(
                    Direction::Horizontal,
                    [Constraint::Min(0), Constraint::Length(20)],
                );
                let [left, right] = layout.areas(area);
                let left_widget = Line::from(vec![
                    Span::raw("DELETE: "),
                    Span::raw("Are you sure? (y) "),
                    Span::from(input),
                ])
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
            _ => todo!(),
        },
        PromptState::Message { value, style } => {
            let type_style = match style {
                PromptStyle::Danger => Style::new().fg(RED.c500),
                PromptStyle::Info => Style::default(),
                _ => todo!()
            };
            let tag = match style {
                PromptStyle::Danger => "Error",
                PromptStyle::Info => "Info",
                _ => todo!()
            };
            let layout = Layout::new(Direction::Horizontal, Constraint::from_fills([1, 1]));
            let [left, right] = layout.areas(area);
            let left_widget = Line::from(format!("{}: {}", tag, value)).style(type_style);
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
    };
}

fn render_help_page(area: Rect, frame: &mut Frame) {
    let text = Paragraph::new(Line::from(
        "This is some awesome help text I need to figure out.",
    ));
    frame.render_widget(text, area);
}

fn render_scopes_page(
    items: Vec<&DirContext>,
    index: usize,
    focus: &ScopesFocus,
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
    frame.render_widget(Paragraph::new(Line::from("Your scopes:")), header);
    frame.render_widget(
        Paragraph::new(Span::styled("-------", Style::new().fg(GRAY.c500))),
        border,
    );
    let index = if items.len() > 0 && matches!(focus, ScopesFocus::List) {
        Some(index)
    } else {
        None
    };
    let main_layout = Layout::new(
        Direction::Vertical,
        [Constraint::Length(1), Constraint::Min(0)],
    );
    let [global_line, list_area] = main_layout.areas(main);

    let global_item = Paragraph::new(Line::from("Global"));
    let global_item_styles = match focus {
        ScopesFocus::List => Style::new(),
        ScopesFocus::Global => Style::new().bg(SLATE.c800),
    };

    frame.render_widget(global_item.style(global_item_styles), global_line);

    let items = items
        .iter()
        .map(|x| {
            let git_remote = x.git_remote.as_ref().map(|x| x.as_str()).unwrap_or("");
            ListItem::new(Line::from(vec![
                Span::raw(x.dir_path.as_str()),
                Span::raw(" "),
                Span::from(git_remote).style(Style::new().fg(SLATE.c500)),
            ]))
        })
        .collect::<Vec<_>>();
    let list = List::new(items)
        .block(Block::new())
        .highlight_style(Style::new().bg(SLATE.c800))
        .highlight_symbol("> ")
        .highlight_spacing(HighlightSpacing::Never);

    let mut state = ListState::default().with_selected(index);
    frame.render_stateful_widget(list, list_area, &mut state);
}

fn render_page_with_prompt(state: &AppState, frame: &mut Frame) {
    let layout = Layout::new(
        Direction::Vertical,
        vec![Constraint::Min(2), Constraint::Length(1)],
    );
    let [page, prompt] = layout.areas(frame.size());
    match &state.ui.page {
        PageState::Dips {
            items,
            index,
            focus,
            scope_id,
        } => {
            let items = items
                .iter()
                .filter_map(|id| state.data.dips.get(id))
                .collect::<Vec<_>>();
            let scope = scope_id.and_then(|id| state.data.scopes.get(&id));
            render_dips_page(scope, items, *index, focus, page, frame);
        }
        PageState::Help => {
            render_help_page(page, frame);
        }
        PageState::Splash => {}
        PageState::Scopes {
            index,
            items,
            focus,
        } => {
            let items = items
                .iter()
                .filter_map(|id| state.data.scopes.get(id))
                .collect::<Vec<_>>();
            render_scopes_page(items, *index, focus, page, frame);
        }
    };
    render_prompt(&state.ui.prompt, prompt, frame);
}

#[derive(Debug)]
enum DataPayload {
    Dips(Vec<DipRowFull>),
    Scopes(Vec<DirContext>),
}

#[derive(Debug)]
enum SearchMode {
    Init,
}

#[derive(Debug)]
enum PromptEvent {
    Focus,
    Defocus,
    Search(SearchMode),
    Confirm(Command),
    Input(char),
    InputBackspace,
    Commit,
    Message {
        msg: &'static str,
        style: PromptStyle,
    },
}

#[derive(Debug)]
enum Action {
    MoveUp,
    MoveDown,
}

#[derive(Debug)]
enum Command {
    Add(String),
    DeleteDip(Uuid),
}

#[derive(Debug)]
enum Event {
    Action(Action),
    Command(Command),
    Prompt(PromptEvent),
    Nav(PageType),
    NavBack,
    LoadData(DataPayload),
    RefetchData(PageType),
    UiTick,
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

    fn handle_page_events(event: &KeyEvent, ctx: &AppState) -> Option<Event> {
        match event.code {
            KeyCode::Char('?') => Some(Event::Nav(PageType::Help)),
            KeyCode::Char('j') | KeyCode::Down => Some(Event::Action(Action::MoveDown)),
            KeyCode::Char('k') | KeyCode::Up => Some(Event::Action(Action::MoveUp)),
            KeyCode::Char(':') => Some(Event::Prompt(PromptEvent::Focus)),
            KeyCode::Char('/') => Some(Event::Prompt(PromptEvent::Search(SearchMode::Init))),
            KeyCode::Char('d') => match &ctx.ui.page {
                PageState::Splash => None,
                PageState::Dips {
                    focus,
                    index,
                    items,
                    ..
                } => match focus {
                    DipsFocus::Scope => None,
                    DipsFocus::List => match items.get(*index) {
                        Some(id) => Some(Event::Prompt(PromptEvent::Confirm(Command::DeleteDip(
                            id.clone(),
                        )))),
                        None => None,
                    },
                },
                PageState::Scopes { .. } => None,
                PageState::Help => None,
            },
            KeyCode::Enter => match &ctx.ui.page {
                PageState::Dips { focus, .. } => match focus {
                    DipsFocus::List => Some(Event::Prompt(PromptEvent::Message {
                        msg: "Enter is not implemented.",
                        style: PromptStyle::Danger,
                    })),
                    DipsFocus::Scope => Some(Event::Nav(PageType::Scopes)),
                },
                PageState::Scopes {
                    items,
                    index,
                    focus,
                } => match focus {
                    ScopesFocus::Global => Some(Event::Nav(PageType::Dips { scope_id: None })),
                    ScopesFocus::List => match items.get(*index) {
                        Some(id) => Some(Event::Nav(PageType::Dips {
                            scope_id: Some(id.to_owned()),
                        })),
                        None => Some(Event::Prompt(PromptEvent::Message {
                            msg: "Could not determine the scope ID",
                            style: PromptStyle::Danger,
                        })),
                    },
                },
                PageState::Splash => None,
                PageState::Help => None,
            },
            _ => None,
        }
    }

    fn handle_prompt_events(event: &KeyEvent, _ctx: &AppState) -> Option<Event> {
        match event.code {
            KeyCode::Esc => Some(Event::Prompt(PromptEvent::Defocus)),
            KeyCode::Backspace => Some(Event::Prompt(PromptEvent::InputBackspace)),
            KeyCode::Char(c) => Some(Event::Prompt(PromptEvent::Input(c))),
            KeyCode::Enter => Some(Event::Prompt(PromptEvent::Commit)),
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
    // fn tag_dip(&self, state: &AppState) {
    //     let item = state
    //         .list_selection_index
    //         .and_then(|x| state.scope_dips.get(x));
    //     match item {
    //         Some(item) => {
    //             let tag = state.search.to_owned();
    //             let id = item.id.to_owned();
    //             let pool = self.db_pool.clone();
    //             let sender = self.sender.clone();
    //             tokio::spawn(async move {
    //                 let mut tx = pool.begin().await.expect("Failed to create transaction");
    //                 tag::create_dip_tag(&mut tx, &id, &tag)
    //                     .await
    //                     .expect("Failed to create a tag for a dip");
    //                 tx.commit().await.expect("Failed to commit a tag for a dip");
    //                 let _ = sender.send(Event::DbResponse(DbResult::Tag));
    //             });
    //         }
    //         None => {
    //             todo!("Send error to the prompt");
    //         }
    //     }
    // }
    //
    // fn remove_dip(&self, state: &AppState) {
    //     let item = state
    //         .list_selection_index
    //         .and_then(|x| state.scope_dips.get(x));
    //     match item {
    //         Some(item) => {
    //             let id = item.id.to_owned();
    //             let pool = self.db_pool.clone();
    //             let sender = self.sender.clone();
    //             tokio::spawn(async move {
    //                 dip::delete(&pool, &id.to_string())
    //                     .await
    //                     .expect("Failed to delete a dip");
    //                 let _ = sender.send(Event::DbResponse(DbResult::Remove));
    //             });
    //         }
    //         None => {
    //             todo!("Send error to the prompt");
    //         }
    //     }
    // }
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
    let query_mgr = QueryManager::new(config.db_pool, tx.clone());

    events.send(Event::Nav(PageType::Dips {
        scope_id: scope.as_ref().map(|x| x.id.clone()),
    }));
    if let Some(scope) = scope {
        events.send(Event::LoadData(DataPayload::Scopes(vec![scope])))
    }

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
            Event::LoadData(result) => match result {
                DataPayload::Dips(items) => {
                    app_state.load_dips_page(items);
                }
                DataPayload::Scopes(items) => {
                    app_state.load_scopes_page(items);
                }
            },
            Event::RefetchData(_) => app_state.ui.page.fetch_data(&query_mgr),
            Event::UiTick => {}
            Event::Action(action) => match action {
                Action::MoveUp => app_state.ui.page.action_move_up(),
                Action::MoveDown => app_state.ui.page.action_move_down(),
            },
            Event::Prompt(action) => match action {
                PromptEvent::Focus => {
                    // TODO: Move out to some function
                    app_state.ui.event_focus = EventFocusMode::Prompt;
                    app_state.ui.prompt.activate_input_state();
                }
                PromptEvent::Defocus => {
                    app_state.ui.event_focus = EventFocusMode::Page;
                    app_state.ui.prompt.activate_default_state();
                }
                PromptEvent::Search(_) => {
                    app_state.ui.event_focus = EventFocusMode::Prompt;
                    app_state.ui.prompt.activate_search_state();
                }
                PromptEvent::Input(c) => {
                    if !app_state.ui.prompt.set_input(c) {
                        app_state.ui.prompt.set_error("Can not type in this mode");
                    }
                }
                PromptEvent::Confirm(cmd) => {
                    app_state.ui.event_focus = EventFocusMode::Prompt;
                    app_state.ui.prompt.activate_confirm_state(cmd);
                }
                PromptEvent::InputBackspace => {
                    app_state.ui.prompt.set_input_backspace();
                }
                PromptEvent::Commit => {
                    app_state.ui.prompt.handle_commit(&events.dispatcher);
                }
                PromptEvent::Message { msg, style } => {
                    app_state.ui.prompt.handle_message(msg, style);
                }
            },
            Event::Command(cmd) => match cmd {
                Command::Add(value) => handle_add_command(&mut app_state.ui, &query_mgr, value),
                Command::DeleteDip(id) => {
                    handle_delete_dip_command(&mut app_state.ui, &query_mgr, id)
                }
            },
            Event::Nav(page) => {
                app_state.ui.navigate(&page);
                app_state.ui.page.fetch_data(&query_mgr);
            }
            Event::NavBack => match app_state.ui.back_page {
                Some(ref page) => {
                    app_state.ui.navigate_back(&page.clone());
                    app_state.ui.page.fetch_data(&query_mgr);
                }
                None => todo!(),
            },
        }
    }

    tui::restore()?;
    Ok(())
}
