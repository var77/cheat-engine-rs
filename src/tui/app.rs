use std::{
    error::Error,
    ops::Index,
    time::{Duration, Instant},
};

use crate::core::{
    self,
    proc::{ProcInfo, get_list},
    scan::{Scan, ScanError, ValueType},
};
use crate::tui::utils::cursor;

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
}

#[derive(Clone, PartialEq)]
pub enum ScanViewWidget {
    ScanResults,
    ValueInput,
    ValueTypeSelect,
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

pub struct App {
    pub proc_filter_input: String,
    pub last_g_press_time: Option<Instant>,
    pub character_index: usize,
    pub input_mode: InputMode,
    pub list_state: ListState,
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
    pub selected_result: Option<core::scan::ScanResult>,
    pub selected_input: Option<SelectedInput>,
    pub value_types: Vec<ValueType>,
    pub value_type_state: ListState,
    scan_view_widgets: Vec<ScanViewWidget>,
    scan_view_selected_widget_index: usize,
    pub scan_view_selected_widget: ScanViewWidget,
    pub app_message: AppMessage,
    pub scan_results_list_state: ListState,
    pub scan_watchlist_list_state: ListState,
}

impl App {
    pub fn new() -> App {
        App {
            last_g_press_time: None,
            input_mode: InputMode::Normal,
            character_index: 0,
            list_state: ListState::default(),
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
            selected_result: None,
            selected_process: None,
            selected_input: None,
            value_types: vec![
                ValueType::U64,
                ValueType::I64,
                ValueType::U32,
                ValueType::I32,
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
            self.list_state.select(Some(0));
        }
        self.current_screen = CurrentScreen::ProcessList;
    }

    fn show_scan_view(&mut self) {
        if self.selected_process.is_none() {
            self.show_process_list();
            return;
        }

        let result = Scan::new(
            self.selected_process.as_ref().unwrap().pid,
            vec![],
            self.value_types
                .get(self.selected_value_type)
                .unwrap_or(&ValueType::U64)
                .clone(),
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
        self.select_widget(ScanViewWidget::ValueInput);
        self.insert_mode_for(SelectedInput::ScanValue);
        cursor::reset_cursor(self);
        self.go_to(CurrentScreen::Scan);
    }

    fn go_to(&mut self, screen: CurrentScreen) {
        self.screen_histroy.push(self.current_screen.clone());
        self.current_screen = screen;
    }

    fn go_back(&mut self) {
        let last_screen = self.screen_histroy.pop();

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
            _ => {
                self.input_mode = InputMode::Normal;
            }
        }
    }

    pub fn select_widget(&mut self, widget: ScanViewWidget) {
        self.scan_view_selected_widget = ScanViewWidget::ValueInput;
        self.scan_view_selected_widget_index = self
            .scan_view_widgets
            .iter()
            .position(|x| x == &widget)
            .unwrap();
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

    fn handle_process_list_event(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('f') => {
                self.insert_mode_for(SelectedInput::ProcessFilter);
            }
            KeyCode::Char('r') => {
                self.show_process_list();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(selected) = self.list_state.selected() {
                    let next = if selected < self.proc_list.len() - 1 {
                        selected + 1
                    } else {
                        0
                    };
                    self.proc_list_vertical_scroll_state =
                        self.proc_list_vertical_scroll_state.position(next);
                    self.list_state.select(Some(next));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(selected) = self.list_state.selected() {
                    let next = if selected > 0 {
                        selected - 1
                    } else {
                        self.proc_list.len() - 1
                    };
                    self.list_state.select(Some(next));
                    self.proc_list_vertical_scroll_state =
                        self.proc_list_vertical_scroll_state.position(next);
                }
            }
            KeyCode::Char('G') => {
                let next = self.proc_list.len() - 1;
                self.proc_list_vertical_scroll_state =
                    self.proc_list_vertical_scroll_state.position(next);
                self.list_state.select(Some(next));
            }
            KeyCode::Char('g') => {
                if let Some(t) = self.last_g_press_time {
                    if t.elapsed() < Duration::from_millis(500) {
                        self.last_g_press_time = None;
                        let next = 0;
                        self.proc_list_vertical_scroll_state =
                            self.proc_list_vertical_scroll_state.position(next);
                        self.list_state.select(Some(next));
                        return;
                    }
                }
                self.last_g_press_time = Some(Instant::now());
            }
            KeyCode::Enter => {
                // select process
                if self.list_state.selected().is_none() {
                    return;
                }

                let selected_process = self.proc_list.get(self.list_state.selected().unwrap());

                if selected_process.is_none() {
                    self.show_process_list();
                    return;
                }

                // Go to scan view
                self.selected_process = Some(selected_process.unwrap().clone());
                self.show_scan_view();
            }
            _ => {}
        }
    }

    fn handle_scan_list_event(&mut self, key_code: KeyCode) {
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
            _ => {}
        }

        match self.scan_view_selected_widget {
            ScanViewWidget::ValueTypeSelect => match key_code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if let Some(selected) = self.value_type_state.selected() {
                        let next = if selected < self.value_types.len() - 1 {
                            selected + 1
                        } else {
                            0
                        };

                        self.value_type_state.select(Some(next));

                        if let Some(scan) = &mut self.scan {
                            scan.set_value_type(self.value_types[next]);
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if let Some(selected) = self.value_type_state.selected() {
                        let next = if selected > 0 {
                            selected - 1
                        } else {
                            self.value_types.len() - 1
                        };

                        if let Some(scan) = &mut self.scan {
                            scan.set_value_type(self.value_types[next]);
                        }
                        self.value_type_state.select(Some(next));
                    }
                }
                _ => {}
            },
            ScanViewWidget::ScanResults | ScanViewWidget::WatchList => match key_code {
                KeyCode::Char('s') => {
                    if let Some(scan) = &mut self.scan {
                        self.app_message =
                            AppMessage::new("Starting new scan...", AppMessageType::Info);
                        if let Err(e) = scan.init() {
                            self.app_message = AppMessage::new(
                                &format!("Error while scanning: {e}"),
                                AppMessageType::Error,
                            );
                        } else if scan.results.len() > 0 {
                            self.scan_results_list_state.select(Some(0));
                            self.app_message = AppMessage::default();
                        } else {
                            self.app_message = AppMessage::default();
                        }
                        self.scan_results_vertical_scroll_state = self
                            .scan_results_vertical_scroll_state
                            .content_length(scan.results.len());
                    }
                }
                KeyCode::Char('r') => {
                    if let Some(scan) = &mut self.scan {
                        self.app_message =
                            AppMessage::new("Refreshing current scan...", AppMessageType::Info);
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
                KeyCode::Char('n') => {
                    if let Some(scan) = &mut self.scan {
                        self.app_message =
                            AppMessage::new("Starting next scan...", AppMessageType::Info);
                        if let Err(e) = scan.next_scan() {
                            self.app_message = AppMessage::new(
                                &format!("Error while scanning: {e}"),
                                AppMessageType::Error,
                            );
                        } else if scan.results.len() > 0 {
                            self.scan_results_list_state.select(Some(0));
                            self.app_message = AppMessage::default();
                        } else {
                            self.app_message = AppMessage::default();
                        }
                        self.scan_results_vertical_scroll_state = self
                            .scan_results_vertical_scroll_state
                            .content_length(scan.results.len());
                    }
                }
                _ => {}
            },
            _ => {}
        }

        if self.scan_view_selected_widget == ScanViewWidget::ScanResults {
            match key_code {
                KeyCode::Char('w') => {
                    if let Some(scan) = &mut self.scan {
                        self.app_message =
                            AppMessage::new("Address added to watchlist", AppMessageType::Info);
                        self.scan_watchlist_vertical_scroll_state = self
                            .scan_watchlist_vertical_scroll_state
                            .content_length(scan.watchlist.len());
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.scan.is_none() {
                        return;
                    }

                    let results = &self.scan.as_ref().unwrap().results;

                    if let Some(selected) = self.scan_results_list_state.selected() {
                        let next = if selected < results.len() - 1 {
                            selected + 1
                        } else {
                            0
                        };

                        self.scan_results_list_state.select(Some(next));
                        self.scan_results_vertical_scroll_state =
                            self.scan_results_vertical_scroll_state.position(next);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.scan.is_none() {
                        return;
                    }

                    let results = &self.scan.as_ref().unwrap().results;
                    if let Some(selected) = self.scan_results_list_state.selected() {
                        let next = if selected > 0 {
                            selected - 1
                        } else {
                            results.len() - 1
                        };

                        self.scan_results_vertical_scroll_state =
                            self.scan_results_vertical_scroll_state.position(next);
                    }
                }
                _ => {}
            };
        }

        if self.scan_view_selected_widget == ScanViewWidget::WatchList {
            match key_code {
                KeyCode::Char('d') => {
                    if let Some(scan) = &mut self.scan {
                        self.app_message =
                            AppMessage::new("Address removed from watchlist", AppMessageType::Info);
                        self.scan_watchlist_vertical_scroll_state = self
                            .scan_watchlist_vertical_scroll_state
                            .content_length(scan.watchlist.len());
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.scan.is_none() {
                        return;
                    }

                    let results = &self.scan.as_ref().unwrap().watchlist;

                    if let Some(selected) = self.scan_watchlist_list_state.selected() {
                        let next = if selected < results.len() - 1 {
                            selected + 1
                        } else {
                            0
                        };

                        self.scan_watchlist_list_state.select(Some(next));
                        self.scan_watchlist_vertical_scroll_state =
                            self.scan_watchlist_vertical_scroll_state.position(next);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.scan.is_none() {
                        return;
                    }

                    let results = &self.scan.as_ref().unwrap().watchlist;
                    if let Some(selected) = self.scan_watchlist_list_state.selected() {
                        let next = if selected > 0 {
                            selected - 1
                        } else {
                            results.len() - 1
                        };

                        self.scan_watchlist_list_state.select(Some(next));
                        self.scan_watchlist_vertical_scroll_state =
                            self.scan_watchlist_vertical_scroll_state.position(next);
                    }
                }
                _ => {}
            }
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
                SelectedInput::ScanValue => {
                    if !self.value_input.is_empty()
                        && scan.set_value_from_str(&self.value_input).is_err()
                    {
                        self.app_message = AppMessage::new(
                            &format!(
                                "Invalid value: {:.10} for type: {}",
                                self.value_input,
                                scan.value_type.to_string(),
                            ),
                            AppMessageType::Error,
                        );
                    } else {
                        self.app_message = AppMessage::default();
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

        if self.selected_input.is_none() || key.code == KeyCode::Esc || key.code == KeyCode::Enter {
            self.accept_input();
            self.input_mode = InputMode::Normal;
            return;
        }

        let current_input;
        match &self.selected_input {
            Some(selected_input) => {
                current_input = match selected_input {
                    SelectedInput::ProcessFilter => &mut self.proc_filter_input,
                    SelectedInput::ScanValue => &mut self.value_input,
                    SelectedInput::StartAddress => &mut self.start_address_input,
                    SelectedInput::EndAddress => &mut self.end_address_input,
                    SelectedInput::ResultValue => &mut self.result_value_input,
                };
            }
            None => {
                return;
            }
        }

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

        match &self.selected_input {
            Some(selected_input) => match selected_input {
                SelectedInput::ProcessFilter => match key.code {
                    KeyCode::Char(_) | KeyCode::Backspace => {
                        self.show_process_list();
                    }
                    _ => {}
                },
                _ => {}
            },
            None => {}
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

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == event::KeyEventKind::Release {
                        continue;
                    }

                    // Special case to handle Ctrl+C early
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            match self.current_screen {
                                CurrentScreen::Exiting => {
                                    self.should_exit = true;
                                }
                                _ => {
                                    self.go_to(CurrentScreen::Exiting);
                                }
                            }
                            continue;
                        }
                        _ => {}
                    }

                    match self.input_mode {
                        InputMode::Normal => self.handle_normal_mode_event(key),
                        InputMode::Insert => self.handle_insert_mode_event(key),
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }
}
