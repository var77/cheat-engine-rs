use std::{
    collections::HashMap,
    error::Error,
    time::{Duration, Instant},
};

use crate::tui::utils::cursor;
use crate::{
    core::{
        self,
        proc::{ProcInfo, get_list},
        scan::{Scan, ScanError, ValueType},
    },
    tui::utils,
};

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    widgets::{ListState, ScrollbarState},
};

#[derive(Clone, PartialEq)]
pub enum CurrentScreen {
    ProcessList,
    Scan,
    ValueEditing,
    Exiting,
}

#[derive(Clone, PartialEq, Debug)]
pub enum SelectedInput {
    ProcessFilter,
    ScanValue,
    StartAddress,
    EndAddress,
    ResultValue,
    ReadSize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScanViewWidget {
    ScanResults,
    ValueInput,
    PermissionsCheckbox,
    ValueTypeSelect,
    ReadSize,
    StartAddressInput,
    EndAddressInput,
    AppMessage,
    WatchList,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessListWidget {
    ProcessList,
    ProcessFilter,
}

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
}

#[derive(Clone, PartialEq, Default)]
pub enum AppMessageType {
    #[default]
    Info,
    Error,
}

#[derive(Clone, Default)]
pub struct AppMessage {
    pub msg: String,
    pub msg_type: AppMessageType,
}

impl AppMessage {
    pub fn new(msg: &str, msg_type: AppMessageType) -> Self {
        AppMessage {
            msg: msg.to_owned(),
            msg_type,
        }
    }
}
#[derive(Clone, PartialEq)]
pub enum AppAction {
    New,
    Refresh,
    Next,
}

// Command pattern for user actions
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    // Navigation commands
    NextWidget,
    PrevWidget,
    GoBack,

    // Input mode commands
    ExitInsertMode,
    AcceptInput,

    // Character input commands
    InsertChar(char),
    DeleteChar,
    MoveCursorLeft,
    MoveCursorRight,

    // Screen commands
    ShowProcessList,
    SelectProcess,

    // Scan commands
    NewScan,
    NextScan,
    RefreshScan,
    ToggleReadWrite,

    // Result commands
    AddToWatchlist,
    RemoveFromWatchlist,
    EditValue,
    CopyValue,

    // List commands
    MoveUp,
    MoveDown,
    MoveToTop,
    MoveToBottom,

    // App commands
    Quit,
    ConfirmQuit,
    CancelQuit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Top,
    Bottom,
}

// Key event wrapper for HashMap keys
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyPress {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyPress {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        KeyPress { code, modifiers }
    }

    pub fn from_key_event(event: KeyEvent) -> Self {
        KeyPress {
            code: event.code,
            modifiers: event.modifiers,
        }
    }
}

// Key bindings system
#[derive(Clone)]
pub struct KeyBindings {
    // Screen-specific bindings
    process_list_normal: HashMap<KeyPress, Command>,
    scan_view_normal: HashMap<KeyPress, Command>,
    exiting_screen: HashMap<KeyPress, Command>,
    insert_mode: HashMap<KeyPress, Command>,
    // Global bindings (work across all screens)
    global: HashMap<KeyPress, Command>,
}

impl KeyBindings {
    pub fn default() -> Self {
        let mut bindings = KeyBindings {
            process_list_normal: HashMap::new(),
            scan_view_normal: HashMap::new(),
            exiting_screen: HashMap::new(),
            insert_mode: HashMap::new(),
            global: HashMap::new(),
        };

        bindings.init_default_bindings();
        bindings
    }

    fn init_default_bindings(&mut self) {
        // Global bindings
        self.global.insert(
            KeyPress::new(KeyCode::Char('q'), KeyModifiers::NONE),
            Command::Quit,
        );
        self.global.insert(
            KeyPress::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Command::Quit,
        );

        // Process list bindings (normal mode)
        self.process_list_normal.insert(
            KeyPress::new(KeyCode::Char('j'), KeyModifiers::NONE),
            Command::MoveDown,
        );
        self.process_list_normal.insert(
            KeyPress::new(KeyCode::Down, KeyModifiers::NONE),
            Command::MoveDown,
        );
        self.process_list_normal.insert(
            KeyPress::new(KeyCode::Char('k'), KeyModifiers::NONE),
            Command::MoveUp,
        );
        self.process_list_normal.insert(
            KeyPress::new(KeyCode::Up, KeyModifiers::NONE),
            Command::MoveUp,
        );
        self.process_list_normal.insert(
            KeyPress::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
            Command::MoveToBottom,
        );
        self.process_list_normal.insert(
            KeyPress::new(KeyCode::Char('r'), KeyModifiers::NONE),
            Command::ShowProcessList,
        );
        self.process_list_normal.insert(
            KeyPress::new(KeyCode::Enter, KeyModifiers::NONE),
            Command::SelectProcess,
        );
        self.process_list_normal.insert(
            KeyPress::new(KeyCode::Tab, KeyModifiers::NONE),
            Command::NextWidget,
        );
        self.process_list_normal.insert(
            KeyPress::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            Command::PrevWidget,
        );

        // Scan view bindings (normal mode)
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('s'), KeyModifiers::NONE),
            Command::NewScan,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('n'), KeyModifiers::NONE),
            Command::NextScan,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('r'), KeyModifiers::NONE),
            Command::RefreshScan,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char(' '), KeyModifiers::NONE),
            Command::ToggleReadWrite,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('w'), KeyModifiers::NONE),
            Command::AddToWatchlist,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('d'), KeyModifiers::NONE),
            Command::RemoveFromWatchlist,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('u'), KeyModifiers::NONE),
            Command::EditValue,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('c'), KeyModifiers::NONE),
            Command::CopyValue,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('y'), KeyModifiers::NONE),
            Command::CopyValue,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Tab, KeyModifiers::NONE),
            Command::NextWidget,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            Command::PrevWidget,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Esc, KeyModifiers::NONE),
            Command::GoBack,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Enter, KeyModifiers::NONE),
            Command::EditValue,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('j'), KeyModifiers::NONE),
            Command::MoveDown,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Down, KeyModifiers::NONE),
            Command::MoveDown,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('k'), KeyModifiers::NONE),
            Command::MoveUp,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Up, KeyModifiers::NONE),
            Command::MoveUp,
        );
        self.scan_view_normal.insert(
            KeyPress::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
            Command::MoveToBottom,
        );

        // Exiting screen bindings
        self.exiting_screen.insert(
            KeyPress::new(KeyCode::Char('y'), KeyModifiers::NONE),
            Command::ConfirmQuit,
        );
        self.exiting_screen.insert(
            KeyPress::new(KeyCode::Char('q'), KeyModifiers::NONE),
            Command::ConfirmQuit,
        );
        self.exiting_screen.insert(
            KeyPress::new(KeyCode::Enter, KeyModifiers::NONE),
            Command::ConfirmQuit,
        );
        self.exiting_screen.insert(
            KeyPress::new(KeyCode::Char('n'), KeyModifiers::NONE),
            Command::CancelQuit,
        );
        self.exiting_screen.insert(
            KeyPress::new(KeyCode::Esc, KeyModifiers::NONE),
            Command::CancelQuit,
        );

        // Insert mode bindings
        self.insert_mode.insert(
            KeyPress::new(KeyCode::Esc, KeyModifiers::NONE),
            Command::ExitInsertMode,
        );
        self.insert_mode.insert(
            KeyPress::new(KeyCode::Enter, KeyModifiers::NONE),
            Command::AcceptInput,
        );
        self.insert_mode.insert(
            KeyPress::new(KeyCode::Backspace, KeyModifiers::NONE),
            Command::DeleteChar,
        );
        self.insert_mode.insert(
            KeyPress::new(KeyCode::Left, KeyModifiers::NONE),
            Command::MoveCursorLeft,
        );
        self.insert_mode.insert(
            KeyPress::new(KeyCode::Right, KeyModifiers::NONE),
            Command::MoveCursorRight,
        );
        self.insert_mode.insert(
            KeyPress::new(KeyCode::Tab, KeyModifiers::NONE),
            Command::NextWidget,
        );
        self.insert_mode.insert(
            KeyPress::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            Command::PrevWidget,
        );
    }

    pub fn get_command(
        &self,
        key_event: KeyEvent,
        screen: &CurrentScreen,
        input_mode: &InputMode,
    ) -> Option<Command> {
        let key_press = KeyPress::from_key_event(key_event);

        // handle exit commands separately as there are matching keys with global keys
        if *screen == CurrentScreen::Exiting {
            return self.exiting_screen.get(&key_press).cloned();
        }

        if let Some(cmd) = self.global.get(&key_press) {
            return Some(cmd.clone());
        }

        match input_mode {
            InputMode::Insert => {
                // In insert mode, check if it's a character input
                if let KeyCode::Char(c) = key_event.code
                    && (key_event.modifiers == KeyModifiers::NONE
                        || key_event.modifiers == KeyModifiers::SHIFT)
                {
                    return Some(Command::InsertChar(c));
                }
                self.insert_mode.get(&key_press).cloned()
            }
            InputMode::Normal => match screen {
                CurrentScreen::ProcessList => self.process_list_normal.get(&key_press).cloned(),
                CurrentScreen::Scan => self.scan_view_normal.get(&key_press).cloned(),
                _ => None,
            },
        }
    }
}

// State management structs
#[derive(Clone)]
pub struct AppState {
    pub current_screen: CurrentScreen,
    pub screen_history: Vec<CurrentScreen>,
    pub should_exit: bool,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            current_screen: CurrentScreen::ProcessList,
            screen_history: vec![],
            should_exit: false,
        }
    }
}

#[derive(Clone)]
pub struct InputBuffers {
    pub process_filter: String,
    pub scan_value: String,
    pub start_address: String,
    pub end_address: String,
    pub result_value: String,
    pub read_size: String,
}

impl InputBuffers {
    pub fn new() -> Self {
        InputBuffers {
            process_filter: String::new(),
            scan_value: String::new(),
            start_address: String::new(),
            end_address: String::new(),
            result_value: String::new(),
            read_size: String::new(),
        }
    }

    pub fn get_mut(&mut self, input: &SelectedInput) -> &mut String {
        match input {
            SelectedInput::ProcessFilter => &mut self.process_filter,
            SelectedInput::ScanValue => &mut self.scan_value,
            SelectedInput::StartAddress => &mut self.start_address,
            SelectedInput::EndAddress => &mut self.end_address,
            SelectedInput::ResultValue => &mut self.result_value,
            SelectedInput::ReadSize => &mut self.read_size,
        }
    }

    pub fn get(&self, input: &SelectedInput) -> &String {
        match input {
            SelectedInput::ProcessFilter => &self.process_filter,
            SelectedInput::ScanValue => &self.scan_value,
            SelectedInput::StartAddress => &self.start_address,
            SelectedInput::EndAddress => &self.end_address,
            SelectedInput::ResultValue => &self.result_value,
            SelectedInput::ReadSize => &self.read_size,
        }
    }

    pub fn len(&self, input: &SelectedInput) -> usize {
        self.get(input).len()
    }
}

#[derive(Clone)]
pub struct ListStates {
    pub proc_list: ListState,
    pub value_type: ListState,
    pub scan_results: ListState,
    pub scan_watchlist: ListState,
}

impl ListStates {
    pub fn new() -> Self {
        ListStates {
            proc_list: ListState::default(),
            value_type: ListState::default(),
            scan_results: ListState::default(),
            scan_watchlist: ListState::default(),
        }
    }
}

#[derive(Clone)]
pub struct ScrollStates {
    pub proc_list_vertical: ScrollbarState,
    pub scan_results_vertical: ScrollbarState,
    pub scan_watchlist_vertical: ScrollbarState,
}

impl ScrollStates {
    pub fn new() -> Self {
        ScrollStates {
            proc_list_vertical: ScrollbarState::default(),
            scan_results_vertical: ScrollbarState::default(),
            scan_watchlist_vertical: ScrollbarState::default(),
        }
    }
}

#[derive(Clone)]
pub struct WidgetSelection {
    pub scan_view_widgets: Vec<ScanViewWidget>,
    pub scan_view_selected_widget_index: usize,
    pub scan_view_selected_widget: ScanViewWidget,
    pub process_list_widgets: Vec<ProcessListWidget>,
    pub process_list_selected_widget_index: usize,
    pub process_list_selected_widget: ProcessListWidget,
}

impl WidgetSelection {
    pub fn new() -> Self {
        WidgetSelection {
            scan_view_widgets: vec![
                ScanViewWidget::ScanResults,
                ScanViewWidget::ValueInput,
                ScanViewWidget::PermissionsCheckbox,
                ScanViewWidget::ValueTypeSelect,
                ScanViewWidget::StartAddressInput,
                ScanViewWidget::EndAddressInput,
                ScanViewWidget::AppMessage,
                ScanViewWidget::WatchList,
            ],
            scan_view_selected_widget: ScanViewWidget::ValueInput,
            scan_view_selected_widget_index: 1,
            process_list_widgets: vec![
                ProcessListWidget::ProcessFilter,
                ProcessListWidget::ProcessList,
            ],
            process_list_selected_widget: ProcessListWidget::ProcessFilter,
            process_list_selected_widget_index: 0,
        }
    }
}

#[derive(Clone)]
pub struct UiState {
    pub input_buffers: InputBuffers,
    pub list_states: ListStates,
    pub scroll_states: ScrollStates,
    pub selected_widgets: WidgetSelection,
    pub input_mode: InputMode,
    pub selected_input: Option<SelectedInput>,
    pub character_index: usize,
    pub last_g_press_time: Option<Instant>,
}

impl UiState {
    pub fn new() -> Self {
        UiState {
            input_buffers: InputBuffers::new(),
            list_states: ListStates::new(),
            scroll_states: ScrollStates::new(),
            selected_widgets: WidgetSelection::new(),
            input_mode: InputMode::Insert,
            selected_input: Some(SelectedInput::ProcessFilter),
            character_index: 0,
            last_g_press_time: None,
        }
    }
}

pub struct App {
    pub state: AppState,
    pub ui: UiState,
    pub scan: Option<core::scan::Scan>,
    pub proc_list: Vec<core::proc::ProcInfo>,
    pub selected_value_type: usize,
    pub selected_process: Option<ProcInfo>,
    pub selected_value: Option<core::scan::ScanResult>,
    pub value_types: Vec<ValueType>,
    pub app_message: AppMessage,
    pub app_action: Option<AppAction>,
    pub key_bindings: KeyBindings,
    pub include_readonly_regions: bool,
}

impl App {
    pub fn new() -> App {
        App {
            state: AppState::new(),
            ui: UiState::new(),
            scan: None,
            proc_list: vec![],
            selected_value_type: 0,
            selected_value: None,
            selected_process: None,
            value_types: vec![
                ValueType::U64,
                ValueType::I64,
                ValueType::U32,
                ValueType::I32,
                ValueType::String,
                ValueType::Hex,
            ],
            app_message: AppMessage::default(),
            app_action: None,
            key_bindings: KeyBindings::default(),
            include_readonly_regions: false,
        }
    }

    fn get_memory_permissions(&self) -> Vec<core::mem::MemoryRegionPerms> {
        if self.include_readonly_regions {
            vec![
                core::mem::MemoryRegionPerms::Write,
                core::mem::MemoryRegionPerms::Read,
            ]
        } else {
            vec![core::mem::MemoryRegionPerms::Write]
        }
    }

    fn show_process_list(&mut self) {
        let filter = if self.ui.input_buffers.process_filter.is_empty() {
            None
        } else {
            Some(self.ui.input_buffers.process_filter.as_str())
        };

        self.proc_list = get_list(filter);
        self.ui.scroll_states.proc_list_vertical = self
            .ui
            .scroll_states
            .proc_list_vertical
            .content_length(self.proc_list.len());
        if !self.proc_list.is_empty() {
            self.ui.list_states.proc_list.select(Some(0));
        }

        self.state.current_screen = CurrentScreen::ProcessList;
        if filter.is_none() {
            self.insert_mode_for(SelectedInput::ProcessFilter);
        }
    }

    fn show_scan_view(&mut self) {
        if self.selected_process.is_none() {
            self.show_process_list();
            return;
        }

        let result = Scan::new(
            self.selected_process.as_ref().unwrap().pid,
            vec![],
            *self
                .value_types
                .get(self.selected_value_type)
                .unwrap_or(&ValueType::U64),
            None,
            None,
            None,
        );

        match result {
            Err(e) => {
                self.app_message = AppMessage::new(
                    &format!("Error initializing scan: {}", e),
                    AppMessageType::Error,
                )
            }
            Ok(scan) => self.scan = Some(scan),
        }

        self.ui.list_states.value_type.select(Some(0));
        self.ui.scroll_states.scan_results_vertical =
            self.ui.scroll_states.scan_results_vertical.position(0);
        self.ui.scroll_states.scan_watchlist_vertical =
            self.ui.scroll_states.scan_watchlist_vertical.position(0);
        self.go_to(CurrentScreen::Scan);
        self.select_widget(ScanViewWidget::ValueInput);
    }

    fn go_to(&mut self, screen: CurrentScreen) {
        self.state
            .screen_history
            .push(self.state.current_screen.clone());
        self.state.current_screen = screen;
    }

    fn go_back(&mut self) {
        let last_screen = self.state.screen_history.pop();

        self.ui.input_mode = InputMode::Normal;
        match last_screen {
            None => {
                self.reset_scan_inputs();
                self.show_process_list();
            }
            Some(screen) => match screen {
                CurrentScreen::ProcessList => {
                    self.ui.input_buffers.process_filter = String::new();
                    self.reset_scan_inputs();
                    self.show_process_list();
                }
                _ => {
                    self.state.current_screen = screen;
                }
            },
        }
    }

    fn reset_scan_inputs(&mut self) {
        self.ui.input_buffers.scan_value = String::new();
        self.ui.input_buffers.start_address = String::new();
        self.ui.input_buffers.end_address = String::new();
        self.ui.input_buffers.read_size = String::new();
        self.include_readonly_regions = false;
        self.scan = None;
        self.selected_process = None;
        self.app_message = AppMessage::default();
    }

    fn enable_auto_input(&mut self) {
        match self.ui.selected_widgets.scan_view_selected_widget {
            ScanViewWidget::ValueInput => self.insert_mode_for(SelectedInput::ScanValue),
            ScanViewWidget::StartAddressInput => self.insert_mode_for(SelectedInput::StartAddress),
            ScanViewWidget::EndAddressInput => self.insert_mode_for(SelectedInput::EndAddress),
            ScanViewWidget::ReadSize => self.insert_mode_for(SelectedInput::ReadSize),
            _ => {
                self.ui.input_mode = InputMode::Normal;
            }
        }
    }

    fn enable_process_list_auto_input(&mut self) {
        match self.ui.selected_widgets.process_list_selected_widget {
            ProcessListWidget::ProcessFilter => self.insert_mode_for(SelectedInput::ProcessFilter),
            ProcessListWidget::ProcessList => {
                self.ui.input_mode = InputMode::Normal;
            }
        }
    }

    pub fn select_widget(&mut self, widget: ScanViewWidget) {
        self.ui.selected_widgets.scan_view_selected_widget_index = self
            .ui
            .selected_widgets
            .scan_view_widgets
            .iter()
            .position(|x| x == &widget)
            .unwrap();
        self.ui.selected_widgets.scan_view_selected_widget =
            self.ui.selected_widgets.scan_view_widgets
                [self.ui.selected_widgets.scan_view_selected_widget_index]
                .clone();

        if widget == ScanViewWidget::WatchList
            && let Some(scan) = &self.scan
            && !scan.watchlist.is_empty()
            && self.ui.list_states.scan_watchlist.selected().is_none()
        {
            self.ui.list_states.scan_watchlist.select(Some(0));
        }

        self.enable_auto_input();
    }

    pub fn insert_mode_for(&mut self, selected_input: SelectedInput) {
        cursor::reset_cursor(self);
        self.ui.input_mode = InputMode::Insert;
        let input_len = self.ui.input_buffers.len(&selected_input);
        self.ui.character_index = input_len;
        self.ui.selected_input = Some(selected_input);
    }

    pub fn next_widget(&mut self) {
        match self.state.current_screen {
            CurrentScreen::Scan => {
                self.ui.selected_widgets.scan_view_selected_widget_index =
                    (self.ui.selected_widgets.scan_view_selected_widget_index + 1)
                        % self.ui.selected_widgets.scan_view_widgets.len();
                self.ui.selected_widgets.scan_view_selected_widget =
                    self.ui.selected_widgets.scan_view_widgets
                        [self.ui.selected_widgets.scan_view_selected_widget_index]
                        .clone();
                self.enable_auto_input();
            }
            CurrentScreen::ProcessList => {
                self.ui.selected_widgets.process_list_selected_widget_index =
                    (self.ui.selected_widgets.process_list_selected_widget_index + 1)
                        % self.ui.selected_widgets.process_list_widgets.len();
                self.ui.selected_widgets.process_list_selected_widget =
                    self.ui.selected_widgets.process_list_widgets
                        [self.ui.selected_widgets.process_list_selected_widget_index]
                        .clone();
                self.enable_process_list_auto_input();
            }
            _ => {}
        }
    }

    pub fn prev_widget(&mut self) {
        match self.state.current_screen {
            CurrentScreen::Scan => {
                let len = self.ui.selected_widgets.scan_view_widgets.len();
                self.ui.selected_widgets.scan_view_selected_widget_index =
                    (self.ui.selected_widgets.scan_view_selected_widget_index + len - 1) % len;
                self.ui.selected_widgets.scan_view_selected_widget =
                    self.ui.selected_widgets.scan_view_widgets
                        [self.ui.selected_widgets.scan_view_selected_widget_index]
                        .clone();
                self.enable_auto_input();
            }
            CurrentScreen::ProcessList => {
                let len = self.ui.selected_widgets.process_list_widgets.len();
                self.ui.selected_widgets.process_list_selected_widget_index =
                    (self.ui.selected_widgets.process_list_selected_widget_index + len - 1) % len;
                self.ui.selected_widgets.process_list_selected_widget =
                    self.ui.selected_widgets.process_list_widgets
                        [self.ui.selected_widgets.process_list_selected_widget_index]
                        .clone();
                self.enable_process_list_auto_input();
            }
            _ => {}
        }
    }

    fn select_process(&mut self) {
        if self.ui.list_states.proc_list.selected().is_none() {
            return;
        }
        let selected_process = self
            .proc_list
            .get(self.ui.list_states.proc_list.selected().unwrap());

        if selected_process.is_none() {
            self.show_process_list();
            return;
        }
        // Go to scan view
        self.selected_process = Some(selected_process.unwrap().clone());
        self.show_scan_view();
    }

    fn check_value_before_scan(&mut self) -> bool {
        if let Some(scan) = &self.scan
            && let Err(e) = scan.value_from_str(&self.ui.input_buffers.scan_value)
        {
            self.app_message = AppMessage::new(&format!("{e}"), AppMessageType::Error);
            self.select_widget(ScanViewWidget::ValueInput);
            return false;
        }

        true
    }

    fn new_scan(&mut self) {
        if !self.check_value_before_scan() {
            return;
        }
        match &mut self.scan {
            None => {}
            Some(scan) => match scan.init() {
                Err(e) => {
                    self.app_message = AppMessage::new(
                        &format!("Error while scanning: {e}"),
                        AppMessageType::Error,
                    );
                }
                Ok(results) => {
                    if !results.is_empty() {
                        self.ui.list_states.scan_results.select(Some(0));
                        self.select_widget(ScanViewWidget::ScanResults);
                    }
                    self.app_message = AppMessage::default();
                }
            },
        }

        if let Some(scan) = &self.scan {
            self.ui.scroll_states.scan_results_vertical = self
                .ui
                .scroll_states
                .scan_results_vertical
                .content_length(scan.results.len());
            self.ui.scroll_states.scan_results_vertical =
                self.ui.scroll_states.scan_results_vertical.position(0);
        }
    }

    fn next_scan(&mut self) {
        if !self.check_value_before_scan() {
            return;
        }
        match &mut self.scan {
            None => {}
            Some(scan) => match scan.next_scan() {
                Err(e) => {
                    self.app_message = AppMessage::new(
                        &format!("Error while scanning: {e}"),
                        AppMessageType::Error,
                    );
                }
                Ok(results) => {
                    if !results.is_empty() {
                        self.ui.list_states.scan_results.select(Some(0));
                        self.select_widget(ScanViewWidget::ScanResults);
                    }
                    self.app_message = AppMessage::default();
                }
            },
        }

        if let Some(scan) = &self.scan {
            self.ui.scroll_states.scan_results_vertical = self
                .ui
                .scroll_states
                .scan_results_vertical
                .content_length(scan.results.len());
            self.ui.scroll_states.scan_results_vertical =
                self.ui.scroll_states.scan_results_vertical.position(0);
        }
    }

    fn refresh_scan(&mut self) {
        if !self.check_value_before_scan() {
            return;
        }
        match &mut self.scan {
            None => {}
            Some(scan) => {
                if let Err(e) = scan.refresh() {
                    self.app_message = AppMessage::new(
                        &format!("Error while scanning: {e}"),
                        AppMessageType::Error,
                    );
                } else {
                    self.app_message = AppMessage::default();
                }
            }
        }
    }

    fn handle_normal_mode_event(&mut self, key: KeyEvent) {
        // Special handling for 'g' key to detect gg
        if key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::NONE {
            if let Some(t) = self.ui.last_g_press_time
                && t.elapsed() < Duration::from_millis(500)
            {
                self.ui.last_g_press_time = None;
                self.handle_command(Command::MoveToTop);
                return;
            }
            self.ui.last_g_press_time = Some(Instant::now());
            return;
        }

        if let Some(cmd) =
            self.key_bindings
                .get_command(key, &self.state.current_screen, &InputMode::Normal)
        {
            self.handle_command(cmd);
        }
    }

    fn accept_input(&mut self) {
        if self.scan.is_none() {
            return;
        }
        let scan = self.scan.as_mut().unwrap();
        if let Some(selected_input) = &self.ui.selected_input {
            match selected_input {
                SelectedInput::ResultValue => {
                    let result = self.selected_value.as_ref().unwrap();
                    match scan.update_value(result.address, &self.ui.input_buffers.result_value) {
                        Err(e) => match e {
                            ScanError::EmptyValue => {
                                self.app_message = AppMessage::new(
                                    "New value can not be empty",
                                    AppMessageType::Error,
                                );
                            }
                            ScanError::InvalidValue => {
                                self.app_message = AppMessage::new(
                                    &format!(
                                        "Invalid value: {:.10} for type: {}",
                                        self.ui.input_buffers.result_value,
                                        scan.value_type.get_string(),
                                    ),
                                    AppMessageType::Error,
                                );
                            }
                            ScanError::Memory(e) => {
                                self.app_message = AppMessage::new(
                                    &format!("Error while updating memory address: {e}",),
                                    AppMessageType::Error,
                                );
                            }
                            _ => {}
                        },
                        Ok(_) => {
                            self.app_action = Some(AppAction::Refresh);
                            self.app_message = AppMessage::new(
                                &format!(
                                    "Value at address 0x{:x} set to {}",
                                    result.address, self.ui.input_buffers.result_value
                                ),
                                AppMessageType::Info,
                            );
                        }
                    }
                    self.go_back();
                }
                SelectedInput::ScanValue => {
                    if !self.ui.input_buffers.scan_value.is_empty()
                        && scan
                            .set_value_from_str(&self.ui.input_buffers.scan_value)
                            .is_err()
                    {
                        self.app_message = AppMessage::new(
                            &format!(
                                "Invalid value: {:.10} for type: {}",
                                self.ui.input_buffers.scan_value,
                                scan.value_type.get_string(),
                            ),
                            AppMessageType::Error,
                        );
                        self.insert_mode_for(SelectedInput::ScanValue);
                    } else {
                        self.app_message = AppMessage::default();
                    }
                }
                SelectedInput::ReadSize => {
                    if self.ui.input_buffers.read_size.is_empty() {
                        scan.set_read_size(None).unwrap();
                        return;
                    }

                    match self.ui.input_buffers.read_size.parse::<usize>() {
                        Err(_) => {
                            self.app_message = AppMessage::new(
                                "Read size should be integer",
                                AppMessageType::Error,
                            );
                            self.insert_mode_for(SelectedInput::ReadSize);
                        }
                        Ok(size) => {
                            if let Err(e) = scan.set_read_size(Some(size)) {
                                self.app_message =
                                    AppMessage::new(&format!("{e}",), AppMessageType::Error);
                                self.insert_mode_for(SelectedInput::ReadSize);
                            } else {
                                self.app_message = AppMessage::default();
                            }
                        }
                    }
                }
                SelectedInput::StartAddress => {
                    if let Err(e) = scan.set_start_address(&self.ui.input_buffers.start_address) {
                        match e {
                            ScanError::InvalidAddress => {
                                self.app_message = AppMessage::new(
                                    &format!(
                                        "Invalid hex value: {:.16}",
                                        self.ui.input_buffers.start_address
                                    ),
                                    AppMessageType::Error,
                                );
                            }
                            ScanError::AddressMismatch => {
                                self.app_message = AppMessage::new(
                                    "Start address should be smaller than end address",
                                    AppMessageType::Error,
                                );
                            }
                            ScanError::Memory(e) => {
                                self.app_message = AppMessage::new(
                                    &format!("Error getting memory regions: {e}"),
                                    AppMessageType::Error,
                                );
                            }
                            _ => {}
                        }
                        self.insert_mode_for(SelectedInput::StartAddress);
                    } else {
                        self.app_message = AppMessage::default();
                    }
                }
                SelectedInput::EndAddress => {
                    if let Err(e) = scan.set_end_address(&self.ui.input_buffers.end_address) {
                        match e {
                            ScanError::InvalidAddress => {
                                self.app_message = AppMessage::new(
                                    &format!(
                                        "Invalid hex value: {:.16}",
                                        self.ui.input_buffers.end_address
                                    ),
                                    AppMessageType::Error,
                                );
                            }
                            ScanError::AddressMismatch => {
                                self.app_message = AppMessage::new(
                                    "End address should be bigger than start address",
                                    AppMessageType::Error,
                                );
                            }
                            ScanError::Memory(e) => {
                                self.app_message = AppMessage::new(
                                    &format!("Error getting memory regions: {e}"),
                                    AppMessageType::Error,
                                );
                            }
                            _ => {}
                        }
                        self.insert_mode_for(SelectedInput::EndAddress);
                    } else {
                        self.app_message = AppMessage::default();
                    }
                }
                _ => {}
            }
        }
    }

    // Command handler
    pub fn handle_command(&mut self, cmd: Command) {
        match cmd {
            // Navigation commands
            Command::NextWidget => {
                // In insert mode, accept input before switching widgets
                if self.ui.input_mode == InputMode::Insert {
                    self.accept_input();
                }
                self.next_widget();
            }
            Command::PrevWidget => {
                if self.ui.input_mode == InputMode::Insert {
                    self.accept_input();
                }
                self.prev_widget();
            }
            Command::GoBack => self.go_back(),

            Command::ExitInsertMode => {
                self.ui.input_mode = InputMode::Normal;
                self.accept_input();
            }
            Command::AcceptInput => {
                self.ui.input_mode = InputMode::Normal;
                self.accept_input();

                // Special handling for process filter
                if let Some(selected_input) = &self.ui.selected_input
                    && selected_input == &SelectedInput::ProcessFilter
                {
                    self.select_process();
                }
            }

            // Character input commands
            Command::InsertChar(c) => {
                if let Some(selected_input) = &self.ui.selected_input {
                    let current_input = self.ui.input_buffers.get_mut(selected_input);
                    cursor::enter_char(current_input, &mut self.ui.character_index, c);

                    // Auto-refresh process list while typing
                    if selected_input == &SelectedInput::ProcessFilter {
                        self.show_process_list();
                    }
                }
            }
            Command::DeleteChar => {
                if let Some(selected_input) = &self.ui.selected_input {
                    let current_input = self.ui.input_buffers.get_mut(selected_input);
                    cursor::delete_char(current_input, &mut self.ui.character_index);

                    // Auto-refresh process list while deleting
                    if selected_input == &SelectedInput::ProcessFilter {
                        self.show_process_list();
                    }
                }
            }
            Command::MoveCursorLeft => {
                if let Some(selected_input) = &self.ui.selected_input {
                    let current_input = self.ui.input_buffers.get_mut(selected_input);
                    cursor::move_cursor_left(current_input, &mut self.ui.character_index);
                }
            }
            Command::MoveCursorRight => {
                if let Some(selected_input) = &self.ui.selected_input {
                    let current_input = self.ui.input_buffers.get_mut(selected_input);
                    cursor::move_cursor_right(current_input, &mut self.ui.character_index);
                }
            }

            // Screen commands
            Command::ShowProcessList => self.show_process_list(),
            Command::SelectProcess => self.select_process(),

            // Scan commands
            Command::NewScan => {
                if self.scan.is_some() {
                    self.app_message =
                        AppMessage::new("Starting new scan...", AppMessageType::Info);
                    self.app_action = Some(AppAction::New);
                }
            }
            Command::NextScan => {
                if self.scan.is_some() {
                    self.app_message =
                        AppMessage::new("Starting next scan...", AppMessageType::Info);
                    self.app_action = Some(AppAction::Next);
                }
            }
            Command::RefreshScan => {
                if self.scan.is_some() {
                    self.app_message =
                        AppMessage::new("Refreshing current scan...", AppMessageType::Info);
                    self.app_action = Some(AppAction::Refresh);
                }
            }
            Command::ToggleReadWrite => {
                if self.ui.selected_widgets.scan_view_selected_widget
                    == ScanViewWidget::PermissionsCheckbox
                {
                    self.include_readonly_regions = !self.include_readonly_regions;
                    let perms = self.get_memory_permissions();
                    if let Some(scan) = &mut self.scan
                        && let Err(e) = scan.set_mem_permissions(perms)
                    {
                        self.app_message = AppMessage::new(
                            &format!("Error setting memory permissions: {}", e),
                            AppMessageType::Error,
                        );
                    }
                }
            }

            // Result commands
            Command::AddToWatchlist => {
                if let Some(scan) = &mut self.scan
                    && self.ui.selected_widgets.scan_view_selected_widget
                        == ScanViewWidget::ScanResults
                    && let Some(selected) = self.ui.list_states.scan_results.selected()
                    && let Some(result) = scan.results.get(selected)
                {
                    scan.add_to_watchlist(result.clone());
                    self.ui.scroll_states.scan_watchlist_vertical = self
                        .ui
                        .scroll_states
                        .scan_watchlist_vertical
                        .content_length(scan.watchlist.len());
                    if self.ui.list_states.scan_watchlist.selected().is_none()
                        && !scan.watchlist.is_empty()
                    {
                        self.ui.list_states.scan_watchlist.select(Some(0));
                    }
                    self.app_message =
                        AppMessage::new("Address added to watchlist", AppMessageType::Info);
                }
            }
            Command::RemoveFromWatchlist => {
                if let Some(scan) = &mut self.scan
                    && self.ui.selected_widgets.scan_view_selected_widget
                        == ScanViewWidget::WatchList
                    && let Some(selected) = self.ui.list_states.scan_watchlist.selected()
                    && let Some(result) = scan.watchlist.get(selected)
                {
                    scan.remove_from_watchlist(result.address);
                    self.ui.scroll_states.scan_watchlist_vertical = self
                        .ui
                        .scroll_states
                        .scan_watchlist_vertical
                        .content_length(scan.watchlist.len());
                    self.app_message =
                        AppMessage::new("Address removed from watchlist", AppMessageType::Info);
                }
            }
            Command::EditValue => match self.ui.selected_widgets.scan_view_selected_widget {
                ScanViewWidget::ValueInput => self.insert_mode_for(SelectedInput::ScanValue),
                ScanViewWidget::StartAddressInput => {
                    self.insert_mode_for(SelectedInput::StartAddress)
                }
                ScanViewWidget::EndAddressInput => self.insert_mode_for(SelectedInput::EndAddress),
                ScanViewWidget::ScanResults | ScanViewWidget::WatchList => {
                    self.selected_value = self.scan.as_ref().and_then(|scan| {
                        let selected_index =
                            match self.ui.selected_widgets.scan_view_selected_widget {
                                ScanViewWidget::ScanResults => {
                                    self.ui.list_states.scan_results.selected()
                                }
                                _ => self.ui.list_states.scan_watchlist.selected(),
                            }?;

                        let list = match self.ui.selected_widgets.scan_view_selected_widget {
                            ScanViewWidget::ScanResults => &scan.results,
                            _ => &scan.watchlist,
                        };

                        list.get(selected_index).cloned()
                    });

                    if let Some(selected_value) = &self.selected_value {
                        if selected_value.is_read_only() {
                            self.app_message = AppMessage::new(
                                "Cannot edit read-only memory region",
                                AppMessageType::Error,
                            );
                        } else {
                            match selected_value.get_string() {
                                Err(e) => {
                                    self.app_message =
                                        AppMessage::new(&format!("{e}"), AppMessageType::Info);
                                }
                                Ok(result_value) => {
                                    self.ui.input_buffers.result_value = result_value;
                                    self.insert_mode_for(SelectedInput::ResultValue);
                                    self.go_to(CurrentScreen::ValueEditing);
                                }
                            }
                        }
                    } else {
                        self.app_message = AppMessage::new(
                            "No result selected for editing.",
                            AppMessageType::Info,
                        );
                    }
                }
                ScanViewWidget::PermissionsCheckbox => {
                    self.handle_command(Command::ToggleReadWrite);
                }
                _ => {}
            },
            Command::CopyValue => {
                if let Some(scan) = &self.scan
                    && (self.ui.selected_widgets.scan_view_selected_widget
                        == ScanViewWidget::ScanResults
                        || self.ui.selected_widgets.scan_view_selected_widget
                            == ScanViewWidget::WatchList)
                {
                    let selected_index = match self.ui.selected_widgets.scan_view_selected_widget {
                        ScanViewWidget::ScanResults => self.ui.list_states.scan_results.selected(),
                        _ => self.ui.list_states.scan_watchlist.selected(),
                    };

                    let list = match self.ui.selected_widgets.scan_view_selected_widget {
                        ScanViewWidget::ScanResults => &scan.results,
                        _ => &scan.watchlist,
                    };

                    if let Some(index) = selected_index
                        && let Some(result) = list.get(index)
                    {
                        match result.get_string() {
                            Ok(value) => {
                                match arboard::Clipboard::new() {
                                    Ok(mut clipboard) => {
                                        if clipboard.set_text(&value).is_ok() {
                                            self.app_message = AppMessage::new(
                                                "Value copied to clipboard",
                                                AppMessageType::Info,
                                            );
                                        } else {
                                            self.app_message = AppMessage::new(
                                                "Failed to copy to clipboard",
                                                AppMessageType::Error,
                                            );
                                        }
                                    }
                                    Err(_) => {
                                        self.app_message = AppMessage::new(
                                            "Failed to access clipboard",
                                            AppMessageType::Error,
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                self.app_message = AppMessage::new(
                                    "Failed to get value",
                                    AppMessageType::Error,
                                );
                            }
                        }
                    } else {
                        self.app_message =
                            AppMessage::new("No result selected", AppMessageType::Info);
                    }
                }
            }

            // List commands
            Command::MoveUp => self.handle_navigate(Direction::Up),
            Command::MoveDown => self.handle_navigate(Direction::Down),
            Command::MoveToTop => self.handle_navigate(Direction::Top),
            Command::MoveToBottom => self.handle_navigate(Direction::Bottom),

            // App commands
            Command::Quit => {
                if self.state.current_screen != CurrentScreen::Exiting {
                    self.go_to(CurrentScreen::Exiting);
                }
            }
            Command::ConfirmQuit => {
                self.state.should_exit = true;
            }
            Command::CancelQuit => {
                self.go_back();
            }
        }
    }

    // Handle navigation (list movement)
    fn handle_navigate(&mut self, dir: Direction) {
        match self.state.current_screen {
            CurrentScreen::ProcessList => {
                // Only navigate the list if the ProcessList widget is selected
                if self.ui.selected_widgets.process_list_selected_widget
                    == ProcessListWidget::ProcessList
                {
                    utils::handle_list_navigation(
                        dir,
                        &mut self.ui.list_states.proc_list,
                        self.proc_list.len(),
                        Some(&mut self.ui.scroll_states.proc_list_vertical),
                        &mut self.ui.last_g_press_time,
                    );
                }
            }
            CurrentScreen::Scan => {
                if let Some(scan) = &mut self.scan {
                    match self.ui.selected_widgets.scan_view_selected_widget {
                        ScanViewWidget::ScanResults => {
                            utils::handle_list_navigation(
                                dir,
                                &mut self.ui.list_states.scan_results,
                                scan.results.len(),
                                Some(&mut self.ui.scroll_states.scan_results_vertical),
                                &mut self.ui.last_g_press_time,
                            );
                        }
                        ScanViewWidget::WatchList => {
                            utils::handle_list_navigation(
                                dir,
                                &mut self.ui.list_states.scan_watchlist,
                                scan.watchlist.len(),
                                Some(&mut self.ui.scroll_states.scan_watchlist_vertical),
                                &mut self.ui.last_g_press_time,
                            );
                        }
                        ScanViewWidget::ValueTypeSelect => {
                            utils::handle_list_navigation(
                                dir,
                                &mut self.ui.list_states.value_type,
                                self.value_types.len(),
                                None,
                                &mut self.ui.last_g_press_time,
                            );
                            // Update value type when selection changes
                            if let Some(selected) = self.ui.list_states.value_type.selected() {
                                if let Err(ScanError::InvalidValue | ScanError::TypeMismatch) = scan
                                    .set_value_type(
                                        self.value_types[selected],
                                        Some(&self.ui.input_buffers.scan_value),
                                    )
                                {
                                    self.app_message = AppMessage::new(
                                        &format!(
                                            "Invalid value: {:.10} for type: {}",
                                            self.ui.input_buffers.result_value,
                                            scan.value_type.get_string(),
                                        ),
                                        AppMessageType::Error,
                                    );
                                }

                                // when string or hex type is selected ReadSize option should be available
                                if scan.value_type == ValueType::String || scan.value_type == ValueType::Hex {
                                    let idx = self
                                        .ui
                                        .selected_widgets
                                        .scan_view_widgets
                                        .iter()
                                        .position(|x| *x == ScanViewWidget::ValueTypeSelect)
                                        .unwrap();
                                    if !self
                                        .ui
                                        .selected_widgets
                                        .scan_view_widgets
                                        .contains(&ScanViewWidget::ReadSize)
                                    {
                                        self.ui
                                            .selected_widgets
                                            .scan_view_widgets
                                            .insert(idx + 1, ScanViewWidget::ReadSize);
                                    }
                                } else if let Some(idx) = self
                                    .ui
                                    .selected_widgets
                                    .scan_view_widgets
                                    .iter()
                                    .position(|x| *x == ScanViewWidget::ReadSize)
                                {
                                    self.ui.selected_widgets.scan_view_widgets.remove(idx);
                                }

                                self.app_message = AppMessage::default();
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_insert_mode_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Use key bindings to get command
        if let Some(cmd) =
            self.key_bindings
                .get_command(key, &self.state.current_screen, &InputMode::Insert)
        {
            self.handle_command(cmd);
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<(), Box<dyn Error>> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();
        self.show_process_list();
        loop {
            if self.state.should_exit {
                return Ok(());
            }

            terminal.draw(|f| super::ui::draw_ui(f, self))?;

            if let Some(app_action) = &mut self.app_action {
                match app_action {
                    AppAction::New => self.new_scan(),
                    AppAction::Next => self.next_scan(),
                    AppAction::Refresh => self.refresh_scan(),
                }
                self.app_action = None;
                continue;
            }

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)?
                && let Event::Key(key) = event::read()?
            {
                if key.kind == event::KeyEventKind::Release {
                    continue;
                }

                // Special case to handle Ctrl+C early
                if let (KeyCode::Char('c'), KeyModifiers::CONTROL) = (key.code, key.modifiers) {
                    if self.state.current_screen == CurrentScreen::Exiting {
                        self.state.should_exit = true;
                    } else {
                        self.go_to(CurrentScreen::Exiting);
                    }
                    continue;
                }

                match self.ui.input_mode {
                    InputMode::Normal => self.handle_normal_mode_event(key),
                    InputMode::Insert => self.handle_insert_mode_event(key),
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }
}
