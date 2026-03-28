//! UI rendering for the Santa TUI.
//!
//! All ratatui widget construction and layout logic lives here,
//! keeping rendering separate from application state.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
    Table, TableState,
};
use ratatui::Frame;

use crate::script_generator::ExecutionMode;
use crate::tui::app::{App, AppPhase};

/// Render the complete TUI frame.
pub fn render(frame: &mut Frame, app: &App) {
    match &app.phase {
        AppPhase::Loading => render_loading(frame),
        AppPhase::Ready | AppPhase::Installing => render_main(frame, app),
        AppPhase::Message(msg) => {
            render_main(frame, app);
            render_message_popup(frame, msg);
        }
    }
}

/// Render a loading spinner / message.
fn render_loading(frame: &mut Frame) {
    let area = frame.area();
    let block = Block::default()
        .title(" 🎅 Santa ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let text = Paragraph::new(vec![
        Line::raw(""),
        Line::raw("  Loading package data..."),
        Line::raw("  Querying package managers for installed packages."),
        Line::raw(""),
    ])
    .block(block);

    frame.render_widget(text, area);
}

/// Render the main interactive view with sources and packages.
fn render_main(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Layout: header(3) + sources(variable) + packages(fill) + footer(3)
    let source_height = 3 + (app.sources.len() as u16).div_ceil(2);
    let chunks = Layout::vertical([
        Constraint::Length(3),              // header
        Constraint::Length(source_height),  // sources
        Constraint::Min(8),                // packages table
        Constraint::Length(3),              // footer
    ])
    .split(area);

    render_header(frame, chunks[0], app);
    render_sources(frame, chunks[1], app);
    render_packages(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
}

/// Render the title bar.
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let installed = app.packages.iter().filter(|p| p.installed).count();
    let total = app.packages.len();
    let missing = total - installed;

    let title = Line::from(vec![
        Span::styled(" 🎅 Santa ", Style::default().fg(Color::Red).bold()),
        Span::raw("│ "),
        Span::styled(format!("{total}"), Style::default().fg(Color::White).bold()),
        Span::raw(" packages, "),
        Span::styled(format!("{installed}"), Style::default().fg(Color::Green).bold()),
        Span::raw(" installed, "),
        Span::styled(
            format!("{missing}"),
            Style::default()
                .fg(if missing > 0 { Color::Yellow } else { Color::Green })
                .bold(),
        ),
        Span::raw(" missing "),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let phase_text = match &app.phase {
        AppPhase::Installing => " Installing...".to_string(),
        _ => String::new(),
    };

    let paragraph = Paragraph::new(phase_text).block(block);
    frame.render_widget(paragraph, area);
}

/// Render the sources panel showing availability.
fn render_sources(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Sources ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.sources.is_empty() {
        let text = Paragraph::new("  No sources configured.");
        frame.render_widget(text, inner);
        return;
    }

    // Render sources in a two-column layout
    let col_width = inner.width / 2;
    for (i, source) in app.sources.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        if row as u16 >= inner.height {
            break;
        }

        let x = inner.x + (col as u16) * col_width;
        let y = inner.y + row as u16;
        let w = col_width.min(inner.width - (col as u16) * col_width);
        let cell_area = Rect::new(x, y, w, 1);

        let (status_icon, status_color) = if source.available {
            ("✓", Color::Green)
        } else {
            ("✗", Color::Red)
        };

        let line = Line::from(vec![
            Span::raw(format!(" {} ", source.emoji)),
            Span::styled(
                format!("{:<10}", source.name),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!(" {status_icon} "),
                Style::default().fg(status_color),
            ),
            Span::styled(
                format!(
                    "{}/{} installed",
                    source.installed_count, source.package_count
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        frame.render_widget(Paragraph::new(line), cell_area);
    }
}

/// Render the main packages table.
fn render_packages(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec![
        Cell::from(""),
        Cell::from(" Package").style(Style::default().bold()),
        Cell::from("Source").style(Style::default().bold()),
        Cell::from("Resolved Name").style(Style::default().bold()),
        Cell::from("Status").style(Style::default().bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .packages
        .iter()
        .enumerate()
        .map(|(i, pkg)| {
            let checkbox = if pkg.installed {
                " "
            } else if app.selected[i] {
                "◉"
            } else {
                "○"
            };

            let (status_text, status_style) = if pkg.installed {
                ("✓ installed", Style::default().fg(Color::Green))
            } else {
                ("✗ missing", Style::default().fg(Color::Yellow))
            };

            let name_display = if pkg.resolved_name != pkg.name {
                pkg.resolved_name.to_string()
            } else {
                "—".to_string()
            };

            Row::new(vec![
                Cell::from(checkbox).style(if app.selected[i] {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
                Cell::from(format!(" {}", pkg.name)),
                Cell::from(format!("{} {}", pkg.source_emoji, pkg.source)),
                Cell::from(name_display).style(Style::default().fg(Color::DarkGray)),
                Cell::from(status_text).style(status_style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Min(20),
        Constraint::Length(18),
        Constraint::Length(20),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(" Packages ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol("▸ ");

    let mut state = TableState::default().with_selected(Some(app.cursor));

    // Render scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);
    let mut scrollbar_state =
        ScrollbarState::new(app.packages.len().saturating_sub(1)).position(app.cursor);

    frame.render_stateful_widget(table, area, &mut state);
    // Render scrollbar in the inner area (skip border)
    let scrollbar_area = Rect {
        x: area.x + area.width - 1,
        y: area.y + 1,
        width: 1,
        height: area.height.saturating_sub(2),
    };
    frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

/// Render the footer with keybinding hints.
fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let selected = app.selected_count();
    let mode_label = match app.execution_mode {
        ExecutionMode::Safe => "generate script",
        ExecutionMode::Execute => "execute install",
    };

    let mut hints = vec![
        Span::styled(" ↑↓ ", Style::default().fg(Color::Cyan).bold()),
        Span::raw("navigate  "),
        Span::styled(" Space ", Style::default().fg(Color::Cyan).bold()),
        Span::raw("select  "),
        Span::styled(" a ", Style::default().fg(Color::Cyan).bold()),
        Span::raw("all missing  "),
        Span::styled(" d ", Style::default().fg(Color::Cyan).bold()),
        Span::raw("deselect  "),
    ];

    if selected > 0 {
        hints.push(Span::styled(
            " i ",
            Style::default().fg(Color::Green).bold(),
        ));
        hints.push(Span::styled(
            format!("{mode_label} ({selected}) "),
            Style::default().fg(Color::Green),
        ));
    }

    hints.push(Span::styled(" r ", Style::default().fg(Color::Cyan).bold()));
    hints.push(Span::raw("refresh  "));
    hints.push(Span::styled(" q ", Style::default().fg(Color::Red).bold()));
    hints.push(Span::raw("quit"));

    let footer = Paragraph::new(Line::from(hints)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
}

/// Render a centered popup message.
fn render_message_popup(frame: &mut Frame, msg: &str) {
    let area = frame.area();
    let popup_width = (msg.len() as u16 + 6).min(area.width - 4);
    let popup_height = 5;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let popup = Paragraph::new(vec![Line::raw(""), Line::raw(format!("  {msg}"))])
        .block(
            Block::default()
                .title(" Result ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(popup, popup_area);
}
