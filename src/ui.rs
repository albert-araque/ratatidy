use crate::app::{App, Tab};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
};

pub fn render(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header/Tabs
            Constraint::Length(3), // Dashboard
            Constraint::Min(0),    // Main Content + Sidebar
            Constraint::Length(3), // Footer
        ])
        .split(frame.size());

    // Main layout: List | Details (conditional)
    let main_constraints = if app.show_details {
        [Constraint::Percentage(60), Constraint::Percentage(40)]
    } else {
        [Constraint::Percentage(100), Constraint::Percentage(0)]
    };

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(main_constraints)
        .split(chunks[2]);

    // Tabs
    let titles = vec!["[1] Media", "[2] Downloads"];
    let index = match app.active_tab {
        Tab::Media => 0,
        Tab::Downloads => 1,
    };
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" ratatidy "))
        .select(index)
        .highlight_style(Style::default().fg(Color::Yellow).bold());
    frame.render_widget(tabs, chunks[0]);

    // Main Content (List)
    let groups = app.current_groups();
    if groups.is_empty() {
        let empty = Paragraph::new("No groups found for this view.")
            .block(Block::default().borders(Borders::ALL).title(" List "));
        frame.render_widget(empty, main_chunks[0]);
    } else {
        let items: Vec<ListItem> = groups
            .iter()
            .map(|group| {
                let mut hardlink_count = 0;
                let mut total_size = 0;
                for node in &group.nodes {
                    if node.has_downloads && node.has_media {
                        hardlink_count += 1;
                    }
                    total_size += node.size;
                }

                let status = format!("[LINK:{}/{}]", hardlink_count, group.nodes.len());

                let size_str = format_size(total_size);

                ListItem::new(format!("{:>10} {} - {}", size_str, status, group.title))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Groups "))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White).bold())
            .highlight_symbol(">> ");

        let mut list_state = ListState::default();
        list_state.select(Some(app.selected_index));
        frame.render_stateful_widget(list, main_chunks[0], &mut list_state);

        // Sidebar / Details (Only if enabled)
        if app.show_details {
            if let Some(group) = groups.get(app.selected_index) {
                let mut lines = vec![
                    ratatui::text::Line::from(format!("Group: {}", group.title))
                        .bold()
                        .yellow(),
                    ratatui::text::Line::from("-".repeat(group.title.len() + 7)).dim(),
                ];

                for node in &group.nodes {
                    let status = if node.has_downloads && node.has_media {
                        " (LINKED) ".fg(Color::Green)
                    } else if node.has_downloads {
                        " (ORPHAN-D) ".fg(Color::Red)
                    } else {
                        " (ORPHAN-M) ".fg(Color::Magenta)
                    };

                    lines.push(ratatui::text::Line::from(vec![
                        "• ".into(),
                        format_size(node.size).into(),
                        status,
                    ]));

                    for path in &node.paths {
                        lines
                            .push(ratatui::text::Line::from(format!("  {}", path.display())).dim());
                    }
                    lines.push(ratatui::text::Line::from(""));
                }

                let details = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title(" Details "))
                    .wrap(ratatui::widgets::Wrap { trim: false });
                frame.render_widget(details, main_chunks[1]);
            }
        }
    }

    // Confirmation Overlay
    if app.show_confirmation {
        let area = centered_rect(60, 40, frame.size());
        frame.render_widget(Clear, area);

        let popup_block = Block::default()
            .title(" PERMANENT DELETE ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red).bold());

        let mut text = vec![
            ratatui::text::Line::from("Select what to purge (Irreversible!):"),
            ratatui::text::Line::from(""),
        ];

        let scope_labels = [
            (
                crate::app::DeleteScope::Downloads,
                " [ ] Delete from Downloads ",
            ),
            (crate::app::DeleteScope::Media, " [ ] Delete from Media "),
            (crate::app::DeleteScope::All, " [ ] Delete from Everywhere "),
        ];

        for (scope, label) in scope_labels {
            if app.available_scopes.contains(&scope) {
                let style = if app.delete_scope == scope {
                    Style::default().bg(Color::Red).fg(Color::White).bold()
                } else {
                    Style::default()
                };
                let mut display_label = label.to_string();
                if app.delete_scope == scope {
                    display_label = display_label.replace("[ ]", "[x]");
                }
                text.push(ratatui::text::Line::from(display_label).style(style));
            }
        }

        text.push(ratatui::text::Line::from(""));
        text.push(
            ratatui::text::Line::from("(Enter to PURGE / Esc to Cancel)")
                .style(Style::default().dim()),
        );

        let paragraph = Paragraph::new(text)
            .block(popup_block)
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    // Footer
    let footer_text = if app.search_active {
        format!(" SEARCH: {}█ (Esc to cancel)", app.search_query)
    } else {
        format!(
            " Tab | i:Info | d:Delete | s:Sort ({:?}) | f:Filter ({:?}) | /:Search | q:Quit ",
            app.sort_by, app.filter
        )
    };

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(if app.search_active {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default()
        });
    frame.render_widget(footer, chunks[3]);

    // Dashboard
    render_dashboard(app, frame, chunks[1]);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_size(bytes: u64) -> String {
    let kb = bytes as f64 / 1024.0;
    let mb = kb / 1024.0;
    let gb = mb / 1024.0;

    if gb >= 1.0 {
        format!("{:.2} GB", gb)
    } else if mb >= 1.0 {
        format!("{:.2} MB", mb)
    } else if kb >= 1.0 {
        format!("{:.2} KB", kb)
    } else {
        format!("{} B", bytes)
    }
}

fn render_dashboard(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let all_groups = match app.active_tab {
        Tab::Media => &app.media_groups,
        Tab::Downloads => &app.download_groups,
    };

    let mut total_files = 0;
    let mut total_size = 0;
    let mut saved_size = 0;

    for group in all_groups {
        for node in &group.nodes {
            total_files += 1;
            if node.has_downloads && node.has_media {
                saved_size += node.size;
            }
            total_size += node.size;
        }
    }

    let stats = format!(
        " Files: {} | Size: {} | Saved: {} ",
        total_files,
        format_size(total_size),
        format_size(saved_size)
    );

    let dashboard = Paragraph::new(stats)
        .block(Block::default().borders(Borders::ALL).title(" Dashboard "))
        .style(Style::default().fg(Color::Yellow).bold())
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(dashboard, area);
}
