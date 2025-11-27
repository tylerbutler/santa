//! UI rendering for the TUI.
//!
//! This module contains all the drawing logic for the terminal interface.

use super::app::{App, Focus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Main draw function that renders the entire UI.
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer/status bar
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_main(frame, chunks[1], app);
    draw_footer(frame, chunks[2], app);

    // Draw help overlay if active
    if app.show_help {
        draw_help_overlay(frame);
    }
}

/// Draw the header with title and last refresh time.
fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let (total, installed, missing) = app.summary();

    let title_spans = vec![
        Span::styled("Santa Dashboard", Style::default().bold()),
        Span::raw(" | "),
        Span::styled(
            format!("{} total", total),
            Style::default().fg(Color::White),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} installed", installed),
            Style::default().fg(Color::Green),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} missing", missing),
            Style::default().fg(Color::Red),
        ),
    ];

    let refresh_info = if app.is_loading {
        Span::styled("Refreshing...", Style::default().fg(Color::Yellow))
    } else {
        Span::styled(
            format!("Last: {}", app.time_since_refresh()),
            Style::default().fg(Color::DarkGray),
        )
    };

    let refresh_width = refresh_info.width();

    // Create the header paragraph with title spans
    let header = Paragraph::new(Line::from(title_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Santa Package Manager "),
    );

    frame.render_widget(header, area);

    // Render refresh time in top-right corner of border
    let refresh_area = Rect::new(
        area.x + area.width.saturating_sub(refresh_width as u16 + 3),
        area.y,
        refresh_width as u16 + 2,
        1,
    );
    frame.render_widget(Paragraph::new(refresh_info), refresh_area);
}

/// Draw the main content area with source and package panels.
fn draw_main(frame: &mut Frame, area: Rect, app: &App) {
    // Split into two columns
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Sources panel
            Constraint::Percentage(65), // Packages panel
        ])
        .split(area);

    draw_sources_panel(frame, chunks[0], app);
    draw_packages_panel(frame, chunks[1], app);
}

/// Draw the sources list panel.
fn draw_sources_panel(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::SourceList;

    let items: Vec<ListItem> = app
        .source_groups
        .iter()
        .enumerate()
        .map(|(i, group)| {
            let is_selected = i == app.selected_source_index;
            let is_expanded = app.is_expanded(i);

            let arrow = if is_expanded { "▾" } else { "▸" };
            let emoji = group.source.emoji();
            let name = group.source.name_str();
            let count = group.total_count();
            let missing = group.missing_count();

            let content = if missing > 0 {
                format!(
                    "{} {} {} ({}, {} missing)",
                    arrow, emoji, name, count, missing
                )
            } else {
                format!("{} {} {} ({})", arrow, emoji, name, count)
            };

            let style = if is_selected && is_focused {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if missing > 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let sources_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" Sources ({}) ", app.source_groups.len())),
    );

    frame.render_widget(sources_list, area);
}

/// Draw the packages panel for the selected source.
fn draw_packages_panel(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::PackageList;

    let border_style = if is_focused {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let (title, items) = if let Some(group) = app.selected_source_group() {
        let title = format!(
            " {} {} ({} total, {} missing) ",
            group.source.emoji(),
            group.source.name_str(),
            group.total_count(),
            group.missing_count()
        );

        let items: Vec<ListItem> = group
            .packages
            .iter()
            .enumerate()
            .map(|(i, pkg)| {
                let is_selected = i == app.selected_package_index;
                let emoji = if pkg.installed { "✅" } else { "❌" };
                let content = format!(" {} {}", emoji, pkg.name);

                let style = if is_selected && is_focused {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else if pkg.installed {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };

                ListItem::new(content).style(style)
            })
            .collect();

        (title, items)
    } else {
        (
            " Packages ".to_string(),
            vec![ListItem::new("No source selected")],
        )
    };

    let packages_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    frame.render_widget(packages_list, area);
}

/// Draw the footer with keybindings.
fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let keybindings = vec![
        Span::styled(" ↑↓ ", Style::default().fg(Color::Yellow)),
        Span::raw("Navigate"),
        Span::styled(" ←→/Tab ", Style::default().fg(Color::Yellow)),
        Span::raw("Switch Panel"),
        Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("Expand"),
        Span::styled(" r ", Style::default().fg(Color::Yellow)),
        Span::raw("Refresh"),
        Span::styled(" ? ", Style::default().fg(Color::Yellow)),
        Span::raw("Help"),
        Span::styled(" q ", Style::default().fg(Color::Yellow)),
        Span::raw("Quit"),
    ];

    let status_line = if let Some(msg) = &app.status_message {
        Line::from(vec![Span::styled(
            msg.clone(),
            Style::default().fg(Color::Yellow),
        )])
    } else {
        Line::from(keybindings)
    };

    let footer = Paragraph::new(status_line)
        .block(Block::default().borders(Borders::ALL))
        .centered();

    frame.render_widget(footer, area);
}

/// Draw the help overlay popup.
fn draw_help_overlay(frame: &mut Frame) {
    let area = frame.area();

    // Create centered popup
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 14.min(area.height.saturating_sub(4));
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the popup area
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled("  Navigation", Style::default().bold())]),
        Line::from("  ↑/k, ↓/j     Move up/down"),
        Line::from("  ←/h, →/l     Switch panels"),
        Line::from("  Tab          Switch focus"),
        Line::from(""),
        Line::from(vec![Span::styled("  Actions", Style::default().bold())]),
        Line::from("  Enter/Space  Toggle expand"),
        Line::from("  r            Refresh data"),
        Line::from("  ?            Toggle this help"),
        Line::from("  q/Esc        Quit"),
    ];

    let help_popup = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help "),
    );

    frame.render_widget(help_popup, popup_area);
}
