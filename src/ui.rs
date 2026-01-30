use crate::app::{App, Tab};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
};

pub fn render(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header/Tabs
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
        .split(chunks[1]);

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
                let mut seeding_count = 0;
                let mut hardlink_count = 0;
                for node in &group.nodes {
                    if node.is_seeding {
                        seeding_count += 1;
                    }
                    if node.has_downloads && node.has_media {
                        hardlink_count += 1;
                    }
                }

                let status = format!(
                    "[SEED:{}/{}] [LINK:{}/{}]",
                    seeding_count,
                    group.nodes.len(),
                    hardlink_count,
                    group.nodes.len()
                );

                ListItem::new(format!("{} - {}", status, group.title))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Groups (Use Arrows/JK) "),
            )
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White).bold())
            .highlight_symbol(">> ");

        let mut state = ListState::default().with_selected(Some(app.selected_index));
        frame.render_stateful_widget(list, main_chunks[0], &mut state);

        // Sidebar / Details (Only if enabled)
        if app.show_details {
            if let Some(selected_group) = groups.get(app.selected_index) {
                let mut detail_text = Vec::new();
                detail_text.push(format!("Group: {}", selected_group.title));
                detail_text.push("-".repeat(selected_group.title.len() + 7));
                detail_text.push(format!("Files in group: {}", selected_group.nodes.len()));
                detail_text.push(String::new());
                detail_text.push("Structure:".to_string());
                for node in &selected_group.nodes {
                    let status = if node.has_downloads && node.has_media {
                        "LINKED"
                    } else {
                        "ORPHAN"
                    };
                    let seeding = if node.is_seeding { "(Seeding)" } else { "" };
                    let file_name = node.paths[0]
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    detail_text.push(format!("• [{}] {} {}", status, file_name, seeding));
                }

                let details = Paragraph::new(detail_text.join("\n")).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Details (i to close) "),
                );
                frame.render_widget(details, main_chunks[1]);
            }
        }
    }

    // Footer
    let footer = Paragraph::new(
        " Tab: Switch POV | i: Details | q: Quit | /: Search | f: Filter | d: Delete | t: Trash ",
    )
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}
