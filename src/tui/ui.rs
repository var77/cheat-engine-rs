use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, HighlightSpacing, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, Wrap,
    },
};

use crate::{
    core::scan::ScanResult,
    tui::app::{App, AppMessageType, CurrentScreen, InputMode, ScanViewWidget, SelectedInput},
};

pub fn draw_process_list(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(100),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    // Render list
    let items: Vec<ListItem> = app
        .proc_list
        .iter()
        .map(|proc| {
            ListItem::new(Line::from(format!("{} - {}", proc.pid, proc.name)))
                .style(Style::new().fg(Color::Green))
        })
        .collect();

    let list_widget = List::new(items)
        .highlight_style(Style::new().bg(Color::Blue).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ")
        .highlight_spacing(HighlightSpacing::Always)
        .block(Block::bordered().title("Process List"));
    frame.render_stateful_widget(list_widget, chunks[0], &mut app.proc_list_state);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        chunks[0],
        &mut app.proc_list_vertical_scroll_state,
    );

    // Render footer
    let input = Paragraph::new(app.proc_filter_input.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Insert => Style::default().fg(Color::Yellow),
        })
        .block(Block::bordered().title("Filter"));
    frame.render_widget(input, chunks[1]);

    match app.input_mode {
        InputMode::Normal => {}
        InputMode::Insert => frame.set_cursor_position(Position::new(
            chunks[1].x + app.character_index as u16 + 1,
            chunks[1].y + 1,
        )),
    }

    // Help text
    let help_text = Line::from(vec![
        Span::from("↑/k: Up  ").fg(Color::Green),
        Span::from("↓/j: Down  ").fg(Color::Green),
        Span::from("f: Filter  ").fg(Color::Green),
        Span::from("r: Refresh  ").fg(Color::Green),
        Span::from("Enter: Select  ").fg(Color::Green),
        Span::from("q: Quit").fg(Color::Green),
    ]);

    let help_bar = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(help_bar, chunks[2]);
}

// fn get_input_style(app: &App, input: SelectedInput) -> Style {
//     if app.input_mode == InputMode::Insert
//         && let Some(selected_input) = &app.selected_input
//     {
//         if *selected_input == input {
//             return Style::default().fg(Color::Yellow);
//         }
//     }
//
//     Style::default()
// }

fn get_active_widget_style(app: &App, widget: ScanViewWidget) -> Style {
    if app.scan_view_selected_widget == widget {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

fn get_message_style(app: &App) -> Style {
    let mut style = match app.app_message.msg_type {
        AppMessageType::Info => Style::default(),
        AppMessageType::Error => Style::default().bg(Color::Red),
    };

    if app.scan_view_selected_widget == ScanViewWidget::AppMessage {
        style = style.fg(Color::Yellow);
    }
    style
}

pub fn draw_scan_screen(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(80),
            Constraint::Percentage(20),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let scan_results_frame = chunks[0];
    let watchlist_rect = chunks[1];
    let scan_view_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(scan_results_frame);
    let scan_results_rect = scan_view_chunks[0];
    let options_rect = scan_view_chunks[1];

    // Render list
    let mut scan_result_items = &vec![];
    let mut watchlist_items = &vec![];
    if let Some(scan) = &app.scan {
        scan_result_items = &scan.results;
        watchlist_items = &scan.watchlist;
    }

    let result_items: Vec<ListItem> = scan_result_items
        .iter()
        .map(|result| {
            ListItem::new(Line::from(format!(
                "0x{:x} | {}",
                result.address,
                result.to_string()
            )))
            .style(Style::new().fg(Color::Green))
        })
        .collect();

    let result_list_widget = List::new(result_items)
        .highlight_style(Style::new().bg(Color::Blue).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ")
        .highlight_spacing(HighlightSpacing::Always)
        .block(
            Block::bordered()
                .title("Scan Results")
                .style(get_active_widget_style(app, ScanViewWidget::ScanResults)),
        );

    frame.render_stateful_widget(
        result_list_widget,
        scan_results_rect,
        &mut app.scan_results_list_state,
    );

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        scan_results_rect,
        &mut app.scan_results_vertical_scroll_state,
    );

    // Watchlist
    let result_items: Vec<ListItem> = watchlist_items
        .iter()
        .map(|result| {
            ListItem::new(Line::from(format!(
                "0x{:x} | {}",
                result.address,
                result.to_string()
            )))
            .style(Style::new().fg(Color::Green))
        })
        .collect();

    let result_list_widget = List::new(result_items)
        .highlight_style(Style::new().bg(Color::Blue).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ")
        .highlight_spacing(HighlightSpacing::Always)
        .block(
            Block::bordered()
                .title("Watchlist")
                .style(get_active_widget_style(app, ScanViewWidget::WatchList)),
        );

    frame.render_stateful_widget(
        result_list_widget,
        watchlist_rect,
        &mut app.scan_watchlist_list_state,
    );

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        watchlist_rect,
        &mut app.scan_watchlist_vertical_scroll_state,
    );
    //
    // Render Options
    let options_view_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(options_rect);

    let value_input = Paragraph::new(app.value_input.as_str())
        .style(get_active_widget_style(app, ScanViewWidget::ValueInput))
        .block(Block::bordered().title("Value"));
    frame.render_widget(value_input, options_view_chunks[0]);

    // Value Type Select
    let items: Vec<ListItem> = app
        .value_types
        .iter()
        .map(|i| ListItem::new(i.get_string()))
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title("Value Type")
                .style(get_active_widget_style(
                    app,
                    ScanViewWidget::ValueTypeSelect,
                )),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, options_view_chunks[1], &mut app.value_type_state);
    //

    let start_address_input = Paragraph::new(app.start_address_input.as_str())
        .style(get_active_widget_style(
            app,
            ScanViewWidget::StartAddressInput,
        ))
        .block(Block::bordered().title("Start Address - hex (optional)"));
    frame.render_widget(start_address_input, options_view_chunks[2]);

    let end_address_input = Paragraph::new(app.end_address_input.as_str())
        .style(get_active_widget_style(
            app,
            ScanViewWidget::EndAddressInput,
        ))
        .block(Block::bordered().title("End Address - hex (optional)"));
    frame.render_widget(end_address_input, options_view_chunks[3]);

    let msg_box = Paragraph::new(app.app_message.msg.as_str())
        .style(get_message_style(app))
        .block(Block::bordered().title("App Message"));
    frame.render_widget(msg_box, options_view_chunks[4]);

    match app.input_mode {
        InputMode::Normal => {}
        InputMode::Insert => {
            let x = options_rect.x + app.character_index as u16 + 1;
            let mut y = 0;
            match &app.selected_input {
                None => {}
                Some(selected_input) => match selected_input {
                    SelectedInput::ScanValue => {
                        y = options_view_chunks[0].y + 1;
                    }
                    SelectedInput::StartAddress => {
                        y = options_view_chunks[2].y + 1;
                    }
                    SelectedInput::EndAddress => {
                        y = options_view_chunks[3].y + 1;
                    }
                    _ => {}
                },
            }

            frame.set_cursor_position(Position::new(x, y));
        }
    }

    // Help text
    let mut help_text_items = vec![Span::from("Tab/Shift+Tab: Change Pane  ").fg(Color::Green)];

    if app.scan_view_selected_widget == ScanViewWidget::ScanResults {
        help_text_items.extend(vec![
            Span::from("s: New Scan  ").fg(Color::Green),
            Span::from("n: Next Scan  ").fg(Color::Green),
            Span::from("w: Add to Watchlist  ").fg(Color::Green),
        ]);
    }

    if app.scan_view_selected_widget == ScanViewWidget::ScanResults
        || app.scan_view_selected_widget == ScanViewWidget::WatchList
    {
        help_text_items.extend(vec![
            Span::from("↑/k: Up  ").fg(Color::Green),
            Span::from("↓/j: Down  ").fg(Color::Green),
            Span::from("r: Refresh  ").fg(Color::Green),
            Span::from("Enter: Update Value  ").fg(Color::Green),
        ]);
    }

    if app.scan_view_selected_widget == ScanViewWidget::WatchList {
        help_text_items.push(Span::from("d: Remove from Watchlist  ").fg(Color::Green));
    }

    help_text_items.push(Span::from("q: Quit").fg(Color::Green));

    let help_bar = Paragraph::new(Line::from(help_text_items))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(help_bar, chunks[2]);
}

pub fn draw_exit_screen(frame: &mut Frame, _app: &mut App) {
    frame.render_widget(Clear, frame.area());

    let popup_block = Block::default()
        .title(" Exit ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    let exit_text = Text::from(vec![
        Line::from(""),
        Line::from(""),
        Line::styled(
            "Would you like to exit? (Y/N)",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        Line::from(""),
        Line::from(""),
    ]);

    let exit_paragraph = Paragraph::new(exit_text)
        .alignment(Alignment::Center)
        .block(popup_block)
        .wrap(Wrap { trim: false });

    let area = centered_rect(50, 30, frame.area());
    frame.render_widget(exit_paragraph, area);
}

pub fn draw_value_editing_screen(frame: &mut Frame, app: &mut App) {
    frame.render_widget(Clear, frame.area());
    let selected_value = app.selected_value.as_ref().unwrap();

    let popup_block = Block::default()
        .title(format!(" Editing - 0x{:x} ", selected_value.address))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    let value_input = Paragraph::new(app.result_value_input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(popup_block);
    let area = centered_rect(50, 30, frame.area());
    frame.set_cursor_position(Position::new(
        area.x + app.character_index as u16 + 1,
        area.y + 1,
    ));
    frame.render_widget(value_input, area);
}

pub fn draw_ui(frame: &mut Frame, app: &mut App) {
    match app.current_screen {
        CurrentScreen::ProcessList => {
            draw_process_list(frame, app);
        }
        CurrentScreen::Scan => {
            draw_scan_screen(frame, app);
        }
        CurrentScreen::ValueEditing => {
            draw_value_editing_screen(frame, app);
        }
        CurrentScreen::Exiting => {
            draw_exit_screen(frame, app);
        }
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}
