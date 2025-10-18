use std::{
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

#[derive(Clone, PartialEq)]
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
    ValueTypeSelect,
    ReadSize,
    StartAddressInput,
    EndAddressInput,
    AppMessage,
    WatchList,
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
}

impl WidgetSelection {
    pub fn new() -> Self {
        WidgetSelection {
            scan_view_widgets: vec![
                ScanViewWidget::ScanResults,
                ScanViewWidget::ValueInput,
                ScanViewWidget::ValueTypeSelect,
                ScanViewWidget::StartAddressInput,
                ScanViewWidget::EndAddressInput,
                ScanViewWidget::AppMessage,
                ScanViewWidget::WatchList,
            ],
            scan_view_selected_widget: ScanViewWidget::ValueInput,
            scan_view_selected_widget_index: 1,
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
            ],
            app_message: AppMessage::default(),
            app_action: None,
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
            .ui.scroll_states.proc_list_vertical
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
        self.state.screen_history.push(self.state.current_screen.clone());
        self.state.current_screen = screen;
    }

    fn go_back(&mut self) {
        let last_screen = self.state.screen_history.pop();

        self.ui.input_mode = InputMode::Normal;
        match last_screen {
            None => {
                self.show_process_list();
            }
            Some(screen) => match screen {
                CurrentScreen::ProcessList => {
                    self.ui.input_buffers.process_filter = String::new();
                    self.show_process_list();
                }
                _ => {
                    self.state.current_screen = screen;
                }
            },
        }
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

    pub fn select_widget(&mut self, widget: ScanViewWidget) {
        self.ui.selected_widgets.scan_view_selected_widget_index = self
            .ui.selected_widgets.scan_view_widgets
            .iter()
            .position(|x| x == &widget)
            .unwrap();
        self.ui.selected_widgets.scan_view_selected_widget =
            self.ui.selected_widgets.scan_view_widgets[self.ui.selected_widgets.scan_view_selected_widget_index].clone();

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
        self.ui.selected_widgets.scan_view_selected_widget_index =
            (self.ui.selected_widgets.scan_view_selected_widget_index + 1) % self.ui.selected_widgets.scan_view_widgets.len();
        self.ui.selected_widgets.scan_view_selected_widget =
            self.ui.selected_widgets.scan_view_widgets[self.ui.selected_widgets.scan_view_selected_widget_index].clone();
        self.enable_auto_input();
    }

    pub fn prev_widget(&mut self) {
        let len = self.ui.selected_widgets.scan_view_widgets.len();
        self.ui.selected_widgets.scan_view_selected_widget_index =
            (self.ui.selected_widgets.scan_view_selected_widget_index + len - 1) % len;
        self.ui.selected_widgets.scan_view_selected_widget =
            self.ui.selected_widgets.scan_view_widgets[self.ui.selected_widgets.scan_view_selected_widget_index].clone();
        self.enable_auto_input();
    }

    fn select_process(&mut self) {
        if self.ui.list_states.proc_list.selected().is_none() {
            return;
        }
        let selected_process = self.proc_list.get(self.ui.list_states.proc_list.selected().unwrap());

        if selected_process.is_none() {
            self.show_process_list();
            return;
        }
        // Go to scan view
        self.selected_process = Some(selected_process.unwrap().clone());
        self.show_scan_view();
    }

    fn handle_process_list_event(&mut self, key_code: KeyCode) {
        utils::handle_list_events(
            key_code,
            &mut self.ui.list_states.proc_list,
            self.proc_list.len(),
            Some(&mut self.ui.scroll_states.proc_list_vertical),
            &mut self.ui.last_g_press_time,
        );
        match key_code {
            KeyCode::Tab | KeyCode::BackTab => match self.ui.input_mode {
                InputMode::Normal => self.insert_mode_for(SelectedInput::ProcessFilter),
                InputMode::Insert => self.ui.input_mode = InputMode::Normal,
            },
            KeyCode::Char('r') => {
                self.show_process_list();
            }
            KeyCode::Enter => {
                self.select_process();
            }
            _ => {}
        };
    }

    fn check_value_before_scan(&mut self) -> bool {
        if let Some(scan) = &self.scan
            && let Err(e) = scan.value_from_str(&self.ui.input_buffers.scan_value) {
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
                .ui.scroll_states.scan_results_vertical
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
                .ui.scroll_states.scan_results_vertical
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

    fn handle_scan_list_event(&mut self, key_code: KeyCode) {
        // Handle list events
        if let Some(scan) = &mut self.scan {
            match self.ui.selected_widgets.scan_view_selected_widget {
                ScanViewWidget::ScanResults => {
                    utils::handle_list_events(
                        key_code,
                        &mut self.ui.list_states.scan_results,
                        scan.results.len(),
                        Some(&mut self.ui.scroll_states.scan_results_vertical),
                        &mut self.ui.last_g_press_time,
                    );
                    if key_code == KeyCode::Char('w')
                        && let Some(selected) = self.ui.list_states.scan_results.selected()
                    {
                        let selected_result = scan.results.get(selected);
                        if let Some(result) = selected_result {
                            scan.add_to_watchlist(result.clone());
                            self.ui.scroll_states.scan_watchlist_vertical = self
                                .ui.scroll_states.scan_watchlist_vertical
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
                }
                ScanViewWidget::WatchList => {
                    utils::handle_list_events(
                        key_code,
                        &mut self.ui.list_states.scan_watchlist,
                        scan.watchlist.len(),
                        Some(&mut self.ui.scroll_states.scan_watchlist_vertical),
                        &mut self.ui.last_g_press_time,
                    );
                    if key_code == KeyCode::Char('d')
                        && let Some(selected) = self.ui.list_states.scan_watchlist.selected()
                    {
                        let selected_result = scan.watchlist.get(selected);
                        if let Some(result) = selected_result {
                            scan.remove_from_watchlist(result.address);
                            self.ui.scroll_states.scan_watchlist_vertical = self
                                .ui.scroll_states.scan_watchlist_vertical
                                .content_length(scan.watchlist.len());
                            self.app_message = AppMessage::new(
                                "Address removed from watchlist",
                                AppMessageType::Info,
                            );
                        }
                    }
                }
                ScanViewWidget::ValueTypeSelect => {
                    utils::handle_list_events(
                        key_code,
                        &mut self.ui.list_states.value_type,
                        self.value_types.len(),
                        None,
                        &mut self.ui.last_g_press_time,
                    );
                    match key_code {
                        KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('k') | KeyCode::Up => {
                            if let Some(selected) = self.ui.list_states.value_type.selected() {
                                if let Err(ScanError::InvalidValue | ScanError::TypeMismatch) = scan.set_value_type(
                                    self.value_types[selected],
                                    Some(&self.ui.input_buffers.scan_value),
                                ) {
                                    self.app_message = AppMessage::new(
                                        &format!(
                                            "Invalid value: {:.10} for type: {}",
                                            self.ui.input_buffers.result_value,
                                            scan.value_type.get_string(),
                                        ),
                                        AppMessageType::Error,
                                    );
                                }

                                // when string type is selected ReadSize option should be
                                // available
                                if scan.value_type == ValueType::String {
                                    let idx = self
                                        .ui.selected_widgets.scan_view_widgets
                                        .iter()
                                        .position(|x| *x == ScanViewWidget::ValueTypeSelect)
                                        .unwrap();
                                    self.ui.selected_widgets.scan_view_widgets
                                        .insert(idx + 1, ScanViewWidget::ReadSize);
                                } else if let Some(idx) = self
                                    .ui.selected_widgets.scan_view_widgets
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
                _ => {}
            }
        }

        // Handle navigation events
        match key_code {
            KeyCode::Esc => {
                self.go_back();
            }
            KeyCode::Tab => {
                self.next_widget();
            }
            KeyCode::BackTab => {
                self.prev_widget();
            }
            KeyCode::Enter => match self.ui.selected_widgets.scan_view_selected_widget {
                ScanViewWidget::ValueInput => self.insert_mode_for(SelectedInput::ScanValue),
                ScanViewWidget::StartAddressInput => {
                    self.insert_mode_for(SelectedInput::StartAddress)
                }
                ScanViewWidget::EndAddressInput => self.insert_mode_for(SelectedInput::EndAddress),
                _ => {}
            },
            KeyCode::Char('s') => {
                if self.scan.is_some() {
                    self.app_message =
                        AppMessage::new("Starting new scan...", AppMessageType::Info);
                    self.app_action = Some(AppAction::New);
                }
            }
            KeyCode::Char('r') => {
                if self.scan.is_some() {
                    self.app_message =
                        AppMessage::new("Refreshing current scan...", AppMessageType::Info);
                    self.app_action = Some(AppAction::Refresh);
                }
            }
            KeyCode::Char('n') => {
                if self.scan.is_some() {
                    self.app_message =
                        AppMessage::new("Starting next scan...", AppMessageType::Info);
                    self.app_action = Some(AppAction::Next);
                }
            }
            _ => {}
        }

        // Handle actions events
        match self.ui.selected_widgets.scan_view_selected_widget {
            ScanViewWidget::ScanResults | ScanViewWidget::WatchList => match key_code {
                KeyCode::Char('u') | KeyCode::Enter => {
                    self.selected_value = self.scan.as_ref().and_then(|scan| {
                        let selected_index = match self.ui.selected_widgets.scan_view_selected_widget {
                            ScanViewWidget::ScanResults => self.ui.list_states.scan_results.selected(),
                            _ => self.ui.list_states.scan_watchlist.selected(),
                        }?;

                        let list = match self.ui.selected_widgets.scan_view_selected_widget {
                            ScanViewWidget::ScanResults => &scan.results,
                            _ => &scan.watchlist,
                        };

                        list.get(selected_index).cloned()
                    });

                    match self.selected_value.as_ref().unwrap().get_string() {
                        Err(e) => {
                            self.app_message =
                                AppMessage::new(&format!("{e}"), AppMessageType::Info);
                        }
                        Ok(result_value) => {
                            if self.selected_value.is_some() {
                                self.ui.input_buffers.result_value = result_value;
                                self.insert_mode_for(SelectedInput::ResultValue);
                                self.go_to(CurrentScreen::ValueEditing);
                            } else {
                                self.app_message = AppMessage::new(
                                    "No result selected for editing.",
                                    AppMessageType::Info,
                                );
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_normal_mode_event(&mut self, key: KeyEvent) {
        if self.state.current_screen != CurrentScreen::Exiting && key.code == KeyCode::Char('q') {
            self.go_to(CurrentScreen::Exiting);
            return;
        }

        match self.state.current_screen {
            CurrentScreen::ProcessList => {
                self.handle_process_list_event(key.code);
            }
            CurrentScreen::Scan => {
                self.handle_scan_list_event(key.code);
            }
            CurrentScreen::Exiting => match key.code {
                KeyCode::Char('y') | KeyCode::Char('q') | KeyCode::Enter => {
                    self.state.should_exit = true;
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.go_back();
                }
                _ => {}
            },
            _ => {}
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
                        && scan.set_value_from_str(&self.ui.input_buffers.scan_value).is_err()
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
                                    &format!("Invalid hex value: {:.16}", self.ui.input_buffers.start_address),
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
                                    &format!("Invalid hex value: {:.16}", self.ui.input_buffers.end_address),
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

    fn handle_insert_mode_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if let Some(selected_input) = &self.ui.selected_input
            && selected_input == &SelectedInput::ProcessFilter
        {
            match key.code {
                KeyCode::Char(_) | KeyCode::Backspace => {
                    self.show_process_list();
                }
                KeyCode::Enter => {
                    self.select_process();
                    return;
                }
                _ => {}
            }
        }

        if self.ui.selected_input.is_none() || key.code == KeyCode::Esc || key.code == KeyCode::Enter {
            self.ui.input_mode = InputMode::Normal;
            self.accept_input();
            return;
        }

        let current_input = match &self.ui.selected_input {
            Some(selected_input) => self.ui.input_buffers.get_mut(selected_input),
            None => {
                return;
            }
        };

        match key.code {
            KeyCode::Char(to_insert) => {
                cursor::enter_char(current_input, &mut self.ui.character_index, to_insert);
            }
            KeyCode::Backspace => {
                cursor::delete_char(current_input, &mut self.ui.character_index);
            }

            KeyCode::Left => cursor::move_cursor_left(current_input, &mut self.ui.character_index),
            KeyCode::Right => cursor::move_cursor_right(current_input, &mut self.ui.character_index),

            KeyCode::Tab => {
                if self.state.current_screen != CurrentScreen::Scan {
                    self.ui.input_mode = InputMode::Normal;
                    return;
                }
                self.accept_input();
                self.next_widget();
            }
            KeyCode::BackTab => {
                if self.state.current_screen != CurrentScreen::Scan {
                    self.ui.input_mode = InputMode::Normal;
                    return;
                }
                self.accept_input();
                self.prev_widget();
            }
            _ => {}
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
