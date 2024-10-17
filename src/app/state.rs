enum PageType {
    Dips,
    Scope,
    Help,
}

enum EventFocusState {
    Page,
    Prompt,
}

struct DipsPageUIState {}

enum PageState {
    Dips { state: DipsPageUIState },
    Scope { index: usize },
}

enum PromptStyle {
    Default,
    Info,
    Danger,
}

enum PromptCtx {
    Command,
    Search,
    Confirm,
    Message,
}

struct PromptState {
    input: String,
    msg: Option<&'static str>,
    style: PromptStyle,
    context: PromptCtx,
}

struct UiState {
    page: PageState,
    prompt: PromptState,
    event_focus: EventFocusState,
}

struct AppState {
    mode: Mode,
    ui: UiState,
    data: DataState,
}

enum Event {
    DataRequest(Request),
    DataResponse(Response),
    Action(Action),
    Input(Input),
    Nav(PageType),
    QuitSignal
}
