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

pub struct App {
    pub proc_filter_input: String,
    pub last_g_press_time: Option<Instant>,
    pub character_index: usize,
    pub input_mode: InputMode,
    pub proc_list_state: ListState,
    should_exit: bool,
    pub proc_list_vertical_scroll_state: ScrollbarState,
    pub scan_results_vertical_scroll_state: ScrollbarState,
    pub scan_watchlist_vertical_scroll_state: ScrollbarState,
    pub screen_histroy: Vec<CurrentScreen>,
    pub current_screen: CurrentScreen,
    pub scan: Option<core::scan::Scan>,
    pub proc_list: Vec<core::proc::ProcInfo>,
    pub value_input: String,
    pub result_value_input: String,
    pub selected_value_type: usize,
    pub selected_process: Option<ProcInfo>,
    pub start_address_input: String,
    pub end_address_input: String,
    pub selected_value: Option<core::scan::ScanResult>,
    pub selected_input: Option<SelectedInput>,
    pub value_types: Vec<ValueType>,
    pub value_type_state: ListState,
    scan_view_widgets: Vec<ScanViewWidget>,
    scan_view_selected_widget_index: usize,
    pub scan_view_selected_widget: ScanViewWidget,
    pub app_message: AppMessage,
    pub app_action: Option<AppAction>,
    pub scan_results_list_state: ListState,
    pub scan_watchlist_list_state: ListState,
    pub read_size_input: String,
}

impl App {
    pub fn new() -> App {
        App {
            last_g_press_time: None,
            input_mode: InputMode::Insert,
            character_index: 0,
            proc_list_state: ListState::default(),
            should_exit: false,
            proc_list_vertical_scroll_state: ScrollbarState::default(),
            scan_results_vertical_scroll_state: ScrollbarState::default(),
            scan_watchlist_vertical_scroll_state: ScrollbarState::default(),
            current_screen: CurrentScreen::ProcessList,
            screen_histroy: vec![],
            scan: None,
            proc_list: vec![],
            proc_filter_input: String::new(),
            value_input: String::new(),
            result_value_input: String::new(),
            selected_value_type: 0,
            start_address_input: String::new(),
            end_address_input: String::new(),
            selected_value: None,
            selected_process: None,
            selected_input: Some(SelectedInput::ProcessFilter),
            value_types: vec![
                ValueType::U64,
                ValueType::I64,
                ValueType::U32,
                ValueType::I32,
                ValueType::String,
            ],
            value_type_state: ListState::default(),
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
            app_message: AppMessage::default(),
            scan_results_list_state: ListState::default(),
            scan_watchlist_list_state: ListState::default(),
            app_action: None,
            read_size_input: String::new(),
        }
    }

    fn show_process_list(&mut self) {
        let filter = if self.proc_filter_input.is_empty() {
            None
        } else {
            Some(self.proc_filter_input.as_str())
        };

        self.proc_list = get_list(filter);
        self.proc_list_vertical_scroll_state = self
            .proc_list_vertical_scroll_state
            .content_length(self.proc_list.len());
        if !self.proc_list.is_empty() {
            self.proc_list_state.select(Some(0));
        }

        self.current_screen = CurrentScreen::ProcessList;
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

        self.value_type_state.select(Some(0));
        self.scan_results_vertical_scroll_state =
            self.scan_results_vertical_scroll_state.position(0);
        self.scan_watchlist_vertical_scroll_state =
            self.scan_watchlist_vertical_scroll_state.position(0);
        self.go_to(CurrentScreen::Scan);
        self.select_widget(ScanViewWidget::ValueInput);
    }

    fn go_to(&mut self, screen: CurrentScreen) {
        self.screen_histroy.push(self.current_screen.clone());
        self.current_screen = screen;
    }

    fn go_back(&mut self) {
        let last_screen = self.screen_histroy.pop();

        self.input_mode = InputMode::Normal;
        match last_screen {
            None => {
                self.show_process_list();
            }
            Some(screen) => match screen {
                CurrentScreen::ProcessList => {
                    self.proc_filter_input = String::new();
                    self.show_process_list();
                }
                _ => {
                    self.current_screen = screen;
                }
            },
        }
    }

    fn enable_auto_input(&mut self) {
        match self.scan_view_selected_widget {
            ScanViewWidget::ValueInput => self.insert_mode_for(SelectedInput::ScanValue),
            ScanViewWidget::StartAddressInput => self.insert_mode_for(SelectedInput::StartAddress),
            ScanViewWidget::EndAddressInput => self.insert_mode_for(SelectedInput::EndAddress),
            ScanViewWidget::ReadSize => self.insert_mode_for(SelectedInput::ReadSize),
            _ => {
                self.input_mode = InputMode::Normal;
            }
        }
    }

    pub fn select_widget(&mut self, widget: ScanViewWidget) {
        self.scan_view_selected_widget_index = self
            .scan_view_widgets
            .iter()
            .position(|x| x == &widget)
            .unwrap();
        self.scan_view_selected_widget =
            self.scan_view_widgets[self.scan_view_selected_widget_index].clone();

        if widget == ScanViewWidget::WatchList
            && let Some(scan) = &self.scan
            && !scan.watchlist.is_empty()
            && self.scan_watchlist_list_state.selected().is_none()
        {
            self.scan_watchlist_list_state.select(Some(0));
        }

        self.enable_auto_input();
    }

    pub fn insert_mode_for(&mut self, selected_input: SelectedInput) {
        cursor::reset_cursor(self);
        self.input_mode = InputMode::Insert;
        let input_len = match selected_input {
            SelectedInput::ProcessFilter => self.proc_filter_input.len(),
            SelectedInput::ScanValue => self.value_input.len(),
            SelectedInput::StartAddress => self.start_address_input.len(),
            SelectedInput::EndAddress => self.end_address_input.len(),
            SelectedInput::ResultValue => self.result_value_input.len(),
            SelectedInput::ReadSize => self.read_size_input.len(),
        };
        self.character_index = input_len;
        self.selected_input = Some(selected_input);
    }

    pub fn next_widget(&mut self) {
        self.scan_view_selected_widget_index =
            (self.scan_view_selected_widget_index + 1) % self.scan_view_widgets.len();
        self.scan_view_selected_widget =
            self.scan_view_widgets[self.scan_view_selected_widget_index].clone();
        self.enable_auto_input();
    }

    pub fn prev_widget(&mut self) {
        let len = self.scan_view_widgets.len();
        self.scan_view_selected_widget_index =
            (self.scan_view_selected_widget_index + len - 1) % len;
        self.scan_view_selected_widget =
            self.scan_view_widgets[self.scan_view_selected_widget_index].clone();
        self.enable_auto_input();
    }

    fn select_process(&mut self) {
        if self.proc_list_state.selected().is_none() {
            return;
        }
        let selected_process = self.proc_list.get(self.proc_list_state.selected().unwrap());

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
            &mut self.proc_list_state,
            self.proc_list.len(),
            Some(&mut self.proc_list_vertical_scroll_state),
            &mut self.last_g_press_time,
        );
        match key_code {
            KeyCode::Tab | KeyCode::BackTab => match self.input_mode {
                InputMode::Normal => self.insert_mode_for(SelectedInput::ProcessFilter),
                InputMode::Insert => self.input_mode = InputMode::Normal,
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
            && let Err(e) = scan.value_from_str(&self.value_input) {
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
                        self.scan_results_list_state.select(Some(0));
                        self.select_widget(ScanViewWidget::ScanResults);
                    }
                    self.app_message = AppMessage::default();
                }
            },
        }

        if let Some(scan) = &self.scan {
            self.scan_results_vertical_scroll_state = self
                .scan_results_vertical_scroll_state
                .content_length(scan.results.len());
            self.scan_results_vertical_scroll_state =
                self.scan_results_vertical_scroll_state.position(0);
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
                        self.scan_results_list_state.select(Some(0));
                        self.select_widget(ScanViewWidget::ScanResults);
                    }
                    self.app_message = AppMessage::default();
                }
            },
        }

        if let Some(scan) = &self.scan {
            self.scan_results_vertical_scroll_state = self
                .scan_results_vertical_scroll_state
                .content_length(scan.results.len());
            self.scan_results_vertical_scroll_state =
                self.scan_results_vertical_scroll_state.position(0);
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
            match self.scan_view_selected_widget {
                ScanViewWidget::ScanResults => {
                    utils::handle_list_events(
                        key_code,
                        &mut self.scan_results_list_state,
                        scan.results.len(),
                        Some(&mut self.scan_results_vertical_scroll_state),
                        &mut self.last_g_press_time,
                    );
                    if key_code == KeyCode::Char('w')
                        && let Some(selected) = self.scan_results_list_state.selected()
                    {
                        let selected_result = scan.results.get(selected);
                        if let Some(result) = selected_result {
                            scan.add_to_watchlist(result.clone());
                            self.scan_watchlist_vertical_scroll_state = self
                                .scan_watchlist_vertical_scroll_state
                                .content_length(scan.watchlist.len());
                            if self.scan_watchlist_list_state.selected().is_none()
                                && !scan.watchlist.is_empty()
                            {
                                self.scan_watchlist_list_state.select(Some(0));
                            }
                            self.app_message =
                                AppMessage::new("Address added to watchlist", AppMessageType::Info);
                        }
                    }
                }
                ScanViewWidget::WatchList => {
                    utils::handle_list_events(
                        key_code,
                        &mut self.scan_watchlist_list_state,
                        scan.watchlist.len(),
                        Some(&mut self.scan_watchlist_vertical_scroll_state),
                        &mut self.last_g_press_time,
                    );
                    if key_code == KeyCode::Char('d')
                        && let Some(selected) = self.scan_watchlist_list_state.selected()
                    {
                        let selected_result = scan.watchlist.get(selected);
                        if let Some(result) = selected_result {
                            scan.remove_from_watchlist(result.address);
                            self.scan_watchlist_vertical_scroll_state = self
                                .scan_watchlist_vertical_scroll_state
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
                        &mut self.value_type_state,
                        self.value_types.len(),
                        None,
                        &mut self.last_g_press_time,
                    );
                    match key_code {
                        KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('k') | KeyCode::Up => {
                            if let Some(selected) = self.value_type_state.selected() {
                                if let Err(ScanError::InvalidValue | ScanError::TypeMismatch) = scan.set_value_type(
                                    self.value_types[selected],
                                    Some(&self.value_input),
                                ) {
                                    self.app_message = AppMessage::new(
                                        &format!(
                                            "Invalid value: {:.10} for type: {}",
                                            self.result_value_input,
                                            scan.value_type.get_string(),
                                        ),
                                        AppMessageType::Error,
                                    );
                                }

                                // when string type is selected ReadSize option should be
                                // available
                                if scan.value_type == ValueType::String {
                                    let idx = self
                                        .scan_view_widgets
                                        .iter()
                                        .position(|x| *x == ScanViewWidget::ValueTypeSelect)
                                        .unwrap();
                                    self.scan_view_widgets
                                        .insert(idx + 1, ScanViewWidget::ReadSize);
                                } else if let Some(idx) = self
                                    .scan_view_widgets
                                    .iter()
                                    .position(|x| *x == ScanViewWidget::ReadSize)
                                {
                                    self.scan_view_widgets.remove(idx);
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
            KeyCode::Enter => match self.scan_view_selected_widget {
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
        match self.scan_view_selected_widget {
            ScanViewWidget::ScanResults | ScanViewWidget::WatchList => match key_code {
                KeyCode::Char('u') | KeyCode::Enter => {
                    self.selected_value = self.scan.as_ref().and_then(|scan| {
                        let selected_index = match self.scan_view_selected_widget {
                            ScanViewWidget::ScanResults => self.scan_results_list_state.selected(),
                            _ => self.scan_watchlist_list_state.selected(),
                        }?;

                        let list = match self.scan_view_selected_widget {
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
                                self.result_value_input = result_value;
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
        if self.current_screen != CurrentScreen::Exiting && key.code == KeyCode::Char('q') {
            self.go_to(CurrentScreen::Exiting);
            return;
        }

        match self.current_screen {
            CurrentScreen::ProcessList => {
                self.handle_process_list_event(key.code);
            }
            CurrentScreen::Scan => {
                self.handle_scan_list_event(key.code);
            }
            CurrentScreen::Exiting => match key.code {
                KeyCode::Char('y') | KeyCode::Char('q') | KeyCode::Enter => {
                    self.should_exit = true;
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
        if let Some(selected_input) = &self.selected_input {
            match selected_input {
                SelectedInput::ResultValue => {
                    let result = self.selected_value.as_ref().unwrap();
                    match scan.update_value(result.address, &self.result_value_input) {
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
                                        self.result_value_input,
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
                                    result.address, self.result_value_input
                                ),
                                AppMessageType::Info,
                            );
                        }
                    }
                    self.go_back();
                }
                SelectedInput::ScanValue => {
                    if !self.value_input.is_empty()
                        && scan.set_value_from_str(&self.value_input).is_err()
                    {
                        self.app_message = AppMessage::new(
                            &format!(
                                "Invalid value: {:.10} for type: {}",
                                self.value_input,
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
                    if self.read_size_input.is_empty() {
                        scan.set_read_size(None).unwrap();
                        return;
                    }

                    match self.read_size_input.parse::<usize>() {
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
                    if let Err(e) = scan.set_start_address(&self.start_address_input) {
                        match e {
                            ScanError::InvalidAddress => {
                                self.app_message = AppMessage::new(
                                    &format!("Invalid hex value: {:.16}", self.start_address_input),
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
                    if let Err(e) = scan.set_end_address(&self.end_address_input) {
                        match e {
                            ScanError::InvalidAddress => {
                                self.app_message = AppMessage::new(
                                    &format!("Invalid hex value: {:.16}", self.end_address_input),
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

        if let Some(selected_input) = &self.selected_input
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

        if self.selected_input.is_none() || key.code == KeyCode::Esc || key.code == KeyCode::Enter {
            self.input_mode = InputMode::Normal;
            self.accept_input();
            return;
        }

        let current_input = match &self.selected_input {
            Some(selected_input) => match selected_input {
                SelectedInput::ProcessFilter => &mut self.proc_filter_input,
                SelectedInput::ScanValue => &mut self.value_input,
                SelectedInput::StartAddress => &mut self.start_address_input,
                SelectedInput::EndAddress => &mut self.end_address_input,
                SelectedInput::ResultValue => &mut self.result_value_input,
                SelectedInput::ReadSize => &mut self.read_size_input,
            },
            None => {
                return;
            }
        };

        match key.code {
            KeyCode::Char(to_insert) => {
                cursor::enter_char(current_input, &mut self.character_index, to_insert);
            }
            KeyCode::Backspace => {
                cursor::delete_char(current_input, &mut self.character_index);
            }

            KeyCode::Left => cursor::move_cursor_left(current_input, &mut self.character_index),
            KeyCode::Right => cursor::move_cursor_right(current_input, &mut self.character_index),

            KeyCode::Tab => {
                if self.current_screen != CurrentScreen::Scan {
                    self.input_mode = InputMode::Normal;
                    return;
                }
                self.accept_input();
                self.next_widget();
            }
            KeyCode::BackTab => {
                if self.current_screen != CurrentScreen::Scan {
                    self.input_mode = InputMode::Normal;
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
            if self.should_exit {
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
                    if self.current_screen == CurrentScreen::Exiting {
                        self.should_exit = true;
                    } else {
                        self.go_to(CurrentScreen::Exiting);
                    }
                    continue;
                }

                match self.input_mode {
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
