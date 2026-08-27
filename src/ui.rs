use crate::app::{App, Modal, ViewMode};
use crate::parser::{ActivityType, Event};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Row, Table,
        Tabs, Wrap,
    },
    Frame,
};
use std::collections::BTreeMap;

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main layout: Header, Tabs, Content, Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Program Info
            Constraint::Length(3), // Tabs & Subheader navigation
            Constraint::Min(10),   // Main Content Pane
            Constraint::Length(2), // Footer & Status
        ])
        .split(size);

    render_header(f, app, chunks[0]);
    render_tabs(f, app, chunks[1]);
    render_content(f, app, chunks[2]);
    render_footer(f, app, chunks[3]);

    // Render active modal on top
    match &app.modal {
        Modal::Help => render_help_modal(f),
        Modal::Search => render_search_modal(f, app),
        Modal::GroupFilter => render_group_filter_modal(f, app),
        Modal::SwitchProfile => render_switch_profile_modal(f, app),
        Modal::EventDetails => render_event_details_modal(f, app),
        Modal::GoToWeek(buf) => render_goto_week_modal(f, buf),
        Modal::None => {}
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(26), // App Title / Logo
            Constraint::Min(30),   // Program info & Profile
            Constraint::Length(28), // Status / Cache info
        ])
        .split(area);

    // Title box
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let title_text = Line::from(vec![
        Span::styled(" 📅 HB SCHEMA ", Style::default().fg(Color::White).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" TUI", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]);
    let title_p = Paragraph::new(title_text)
        .block(title_block)
        .alignment(Alignment::Center);
    f.render_widget(title_p, header_chunks[0]);

    // Program info box
    let program_info = if let Some(schedule) = &app.schedule {
        let p_info = &schedule.metadata.program_info;
        if !p_info.is_empty() {
            p_info.clone()
        } else {
            app.active_profile.name.clone()
        }
    } else {
        app.active_profile.name.clone()
    };

    let p_clean = program_info.replace('\n', " ").replace('\t', " ");
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let info_lines = vec![
        Line::from(vec![
            Span::styled("Profil: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(&app.active_profile.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(p_clean, Style::default().fg(Color::Gray)),
        ]),
    ];
    let info_p = Paragraph::new(info_lines).block(info_block);
    f.render_widget(info_p, header_chunks[1]);

    // Cache / Status info
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let fetch_badge = if let Some(info) = &app.fetch_info {
        if info.contains("Live") {
            Span::styled(format!(" 🟢 {} ", info), Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(format!(" 🟡 {} ", info), Style::default().fg(Color::Black).bg(Color::Yellow))
        }
    } else {
        Span::raw(" Laddar...")
    };

    let status_p = Paragraph::new(Line::from(vec![fetch_badge]))
        .block(status_block)
        .alignment(Alignment::Right);
    f.render_widget(status_p, header_chunks[2]);
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(45), // Tabs
            Constraint::Max(45), // Filter tags
        ])
        .split(area);

    let tab_titles = vec![
        " [1] Veckovy ",
        " [2] Alla händelser ",
        " [3] Dagvy ",
        " [4] Kurser ",
    ];

    let selected_tab = match app.view_mode {
        ViewMode::Week => 0,
        ViewMode::Timeline => 1,
        ViewMode::Day => 2,
        ViewMode::Courses => 3,
    };

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .select(selected_tab)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .divider(symbols::DOT);

    f.render_widget(tabs, chunks[0]);

    // Active filters summary
    let mut filter_spans = Vec::new();

    if !app.selected_groups.is_empty() {
        filter_spans.push(Span::styled(" 👥 Grupp: ", Style::default().fg(Color::Yellow)));
        filter_spans.push(Span::styled(format!("{} ", app.selected_groups_display()), Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)));
        filter_spans.push(Span::raw(" "));
    }

    if !app.search_query.is_empty() {
        filter_spans.push(Span::styled(" 🔍 Sök: ", Style::default().fg(Color::Cyan)));
        filter_spans.push(Span::styled(format!("\"{}\" ", app.search_query), Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)));
        filter_spans.push(Span::raw(" "));
    }

    if let Some(ref act) = app.activity_filter {
        filter_spans.push(Span::styled(format!(" {} ", act.display_name()), Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)));
    }

    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let filter_p = Paragraph::new(Line::from(filter_spans))
        .block(filter_block)
        .alignment(Alignment::Right);

    f.render_widget(filter_p, chunks[1]);
}

fn render_content(f: &mut Frame, app: &mut App, area: Rect) {
    match app.view_mode {
        ViewMode::Week => render_week_view(f, app, area),
        ViewMode::Timeline => render_timeline_view(f, app, area),
        ViewMode::Day => render_day_view(f, app, area),
        ViewMode::Courses => render_courses_view(f, app, area),
    }
}

fn render_week_view(f: &mut Frame, app: &mut App, area: Rect) {
    let week_info = app.get_current_week_info();
    let (week_num, week_year, week_str) = if let Some(w) = week_info {
        (w.week_number, w.year, format!("Vecka {}, {}", w.week_number, w.year))
    } else {
        (0, 2026, "Ingen vecka".to_string())
    };

    let events = app.get_filtered_events_for_week(week_num, week_year);
    let total_weeks = app.schedule.as_ref().map(|s| s.weeks.len()).unwrap_or(1);
    let week_idx_display = format!("{}/{}", app.selected_week_index + 1, total_weeks);

    // Split area: Left = Event Table, Right = Selected Event Inspector
    let main_chunks = if area.width >= 100 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area)
    };

    let table_title = format!(" ◄ [p/h] {} ({}) [n/l] ► ({} st) ", week_str, week_idx_display, events.len());

    let list_block = Block::default()
        .title(Span::styled(table_title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let rows: Vec<Row> = events.iter().enumerate().map(|(idx, ev)| {
        let is_selected = idx == app.selected_event_index;

        let style = if is_selected {
            Style::default().fg(Color::White).bg(Color::Rgb(30, 60, 90)).add_modifier(Modifier::BOLD)
        } else if idx % 2 == 0 {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        let act_style = match ev.activity_type {
            ActivityType::Lecture => Style::default().fg(Color::Cyan),
            ActivityType::Exercise => Style::default().fg(Color::Green),
            ActivityType::Lab => Style::default().fg(Color::Magenta),
            ActivityType::Exam => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ActivityType::Seminar => Style::default().fg(Color::Blue),
            ActivityType::Other(_) => Style::default().fg(Color::Yellow),
        };

        let day_badge = format!("{} {}", ev.day, ev.date);
        let rooms_display = if ev.rooms.is_empty() { "-".to_string() } else { ev.rooms.join(", ") };
        let sign_display = if ev.signatures.is_empty() { "-".to_string() } else { ev.signatures.join(", ") };

        Row::new(vec![
            Span::styled(day_badge, Style::default().fg(Color::Yellow)),
            Span::styled(&ev.time_span, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} {}", ev.activity_type.icon(), ev.activity_type.display_name()), act_style),
            Span::styled(&ev.course_code, Style::default().fg(Color::White)),
            Span::styled(rooms_display, Style::default().fg(Color::LightCyan)),
            Span::styled(sign_display, Style::default().fg(Color::LightYellow)),
            Span::styled(&ev.moment, Style::default().fg(Color::Gray)),
        ]).style(style)
    }).collect();

    let header_row = Row::new(vec![
        Span::styled("Dag/Datum", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Tid", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Typ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Kurs", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Lokal", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Sign", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Moment", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]).style(Style::default().bg(Color::Rgb(20, 25, 35)));

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(13),
            Constraint::Length(15),
            Constraint::Length(18),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Min(20),
        ],
    )
    .header(header_row)
    .block(list_block)
    .highlight_symbol("▶ ");

    f.render_widget(table, main_chunks[0]);

    let selected_event = events.get(app.selected_event_index).copied();
    render_event_inspector(f, selected_event, main_chunks[1]);
}

fn render_timeline_view(f: &mut Frame, app: &mut App, area: Rect) {
    let events = app.get_all_filtered_events();

    let main_chunks = if area.width >= 100 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area)
    };

    let title = format!(" Alla händelser ({} st totalt) ", events.len());
    let list_block = Block::default()
        .title(Span::styled(title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let rows: Vec<Row> = events.iter().enumerate().map(|(idx, ev)| {
        let is_selected = idx == app.selected_event_index;

        let style = if is_selected {
            Style::default().fg(Color::White).bg(Color::Rgb(30, 60, 90)).add_modifier(Modifier::BOLD)
        } else if idx % 2 == 0 {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        let act_style = match ev.activity_type {
            ActivityType::Lecture => Style::default().fg(Color::Cyan),
            ActivityType::Exercise => Style::default().fg(Color::Green),
            ActivityType::Lab => Style::default().fg(Color::Magenta),
            ActivityType::Exam => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ActivityType::Seminar => Style::default().fg(Color::Blue),
            ActivityType::Other(_) => Style::default().fg(Color::Yellow),
        };

        let week_day_badge = format!("V{:02} {} {}", ev.week, ev.day, ev.date);
        let rooms_display = if ev.rooms.is_empty() { "-".to_string() } else { ev.rooms.join(", ") };
        let sign_display = if ev.signatures.is_empty() { "-".to_string() } else { ev.signatures.join(", ") };

        Row::new(vec![
            Span::styled(week_day_badge, Style::default().fg(Color::Yellow)),
            Span::styled(&ev.time_span, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} {}", ev.activity_type.icon(), ev.activity_type.display_name()), act_style),
            Span::styled(&ev.course_code, Style::default().fg(Color::White)),
            Span::styled(rooms_display, Style::default().fg(Color::LightCyan)),
            Span::styled(sign_display, Style::default().fg(Color::LightYellow)),
            Span::styled(&ev.moment, Style::default().fg(Color::Gray)),
        ]).style(style)
    }).collect();

    let header_row = Row::new(vec![
        Span::styled("Vecka/Datum", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Tid", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Typ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Kurs", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Lokal", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Sign", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Moment", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]).style(Style::default().bg(Color::Rgb(20, 25, 35)));

    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(13),
            Constraint::Length(15),
            Constraint::Length(18),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Min(20),
        ],
    )
    .header(header_row)
    .block(list_block)
    .highlight_symbol("▶ ");

    f.render_widget(table, main_chunks[0]);

    let selected_event = events.get(app.selected_event_index).copied();
    render_event_inspector(f, selected_event, main_chunks[1]);
}

fn render_day_view(f: &mut Frame, app: &mut App, area: Rect) {
    let days = app.get_unique_days();
    let current_day = days.get(app.selected_day_index).cloned().unwrap_or_default();
    let events = app.get_current_view_events();

    let main_chunks = if area.width >= 100 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area)
    };

    let title = format!(" ◄ [h/Left] Dag: {} ({}/{}) [l/Right] ► ({} händelser) ",
        current_day, app.selected_day_index + 1, days.len(), events.len());

    let list_block = Block::default()
        .title(Span::styled(title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let rows: Vec<Row> = events.iter().enumerate().map(|(idx, ev)| {
        let is_selected = idx == app.selected_event_index;

        let style = if is_selected {
            Style::default().fg(Color::White).bg(Color::Rgb(30, 60, 90)).add_modifier(Modifier::BOLD)
        } else if idx % 2 == 0 {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        let act_style = match ev.activity_type {
            ActivityType::Lecture => Style::default().fg(Color::Cyan),
            ActivityType::Exercise => Style::default().fg(Color::Green),
            ActivityType::Lab => Style::default().fg(Color::Magenta),
            ActivityType::Exam => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ActivityType::Seminar => Style::default().fg(Color::Blue),
            ActivityType::Other(_) => Style::default().fg(Color::Yellow),
        };

        let rooms_display = if ev.rooms.is_empty() { "-".to_string() } else { ev.rooms.join(", ") };
        let sign_display = if ev.signatures.is_empty() { "-".to_string() } else { ev.signatures.join(", ") };

        Row::new(vec![
            Span::styled(&ev.time_span, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} {}", ev.activity_type.icon(), ev.activity_type.display_name()), act_style),
            Span::styled(&ev.course_code, Style::default().fg(Color::White)),
            Span::styled(rooms_display, Style::default().fg(Color::LightCyan)),
            Span::styled(sign_display, Style::default().fg(Color::LightYellow)),
            Span::styled(&ev.moment, Style::default().fg(Color::Gray)),
        ]).style(style)
    }).collect();

    let header_row = Row::new(vec![
        Span::styled("Tid", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Typ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Kurs", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Lokal", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Sign", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Moment", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]).style(Style::default().bg(Color::Rgb(20, 25, 35)));

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(16),
            Constraint::Length(20),
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Min(20),
        ],
    )
    .header(header_row)
    .block(list_block)
    .highlight_symbol("▶ ");

    f.render_widget(table, main_chunks[0]);

    let selected_event = events.get(app.selected_event_index).copied();
    render_event_inspector(f, selected_event, main_chunks[1]);
}

fn render_courses_view(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let all_events = app.get_all_filtered_events();
    let mut courses_map: BTreeMap<String, Vec<&Event>> = BTreeMap::new();
    for ev in all_events {
        courses_map.entry(ev.course.clone()).or_default().push(ev);
    }

    let course_keys: Vec<String> = courses_map.keys().cloned().collect();

    let list_block = Block::default()
        .title(Span::styled(format!(" Kurser i schemat ({} st) ", course_keys.len()), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let items: Vec<ListItem> = course_keys.iter().enumerate().map(|(idx, c_name)| {
        let is_selected = idx == app.selected_course_index;
        let count = courses_map.get(c_name).map(|v| v.len()).unwrap_or(0);
        let style = if is_selected {
            Style::default().fg(Color::White).bg(Color::Rgb(30, 60, 90)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        ListItem::new(vec![
            Line::from(vec![
                Span::styled(format!("{}. ", idx + 1), Style::default().fg(Color::Cyan)),
                Span::styled(c_name, Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(format!("   Totalt {} schematillfällen", count), Style::default().fg(Color::Gray)),
            ]),
        ]).style(style)
    }).collect();

    let list = List::new(items).block(list_block);
    f.render_widget(list, chunks[0]);

    let detail_block = Block::default()
        .title(Span::styled(" Kursstatistik & Sammanfattning ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    if let Some(target_course) = course_keys.get(app.selected_course_index) {
        if let Some(ev_list) = courses_map.get(target_course) {
            let mut lectures = 0;
            let mut exercises = 0;
            let mut labs = 0;
            let mut exams = 0;
            let mut rooms_set: Vec<String> = Vec::new();
            let mut signs_set: Vec<String> = Vec::new();

            for ev in ev_list {
                match ev.activity_type {
                    ActivityType::Lecture => lectures += 1,
                    ActivityType::Exercise => exercises += 1,
                    ActivityType::Lab => labs += 1,
                    ActivityType::Exam => exams += 1,
                    _ => {}
                }
                for r in &ev.rooms {
                    if !rooms_set.contains(r) {
                        rooms_set.push(r.clone());
                    }
                }
                for s in &ev.signatures {
                    if !signs_set.contains(s) {
                        signs_set.push(s.clone());
                    }
                }
            }

            let first_date = ev_list.first().map(|e| e.full_date.as_str()).unwrap_or("-");
            let last_date = ev_list.last().map(|e| e.full_date.as_str()).unwrap_or("-");

            let lines = vec![
                Line::from(vec![
                    Span::styled("Kurs: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(target_course, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("Period: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} till {}", first_date, last_date), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Schematillfällen: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} st", ev_list.len()), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("Aktivitetsfördelning:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("  • Föreläsningar: ", Style::default().fg(Color::Cyan)),
                    Span::styled(format!("{} st", lectures), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("  • Övningar / Lektioner: ", Style::default().fg(Color::Green)),
                    Span::styled(format!("{} st", exercises), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("  • Laborationer: ", Style::default().fg(Color::Magenta)),
                    Span::styled(format!("{} st", labs), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("  • Tentamen / Prov: ", Style::default().fg(Color::Red)),
                    Span::styled(format!("{} st", exams), Style::default().fg(Color::White)),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("Lokaler som används:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {}", rooms_set.join(", ")), Style::default().fg(Color::LightCyan)),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("Lärare / Signaturer:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {}", signs_set.join(", ")), Style::default().fg(Color::LightYellow)),
                ]),
            ];

            let p = Paragraph::new(lines).block(detail_block).wrap(Wrap { trim: true });
            f.render_widget(p, chunks[1]);
            return;
        }
    }

    let p = Paragraph::new("Ingen kurs vald.").block(detail_block);
    f.render_widget(p, chunks[1]);
}

fn render_event_inspector(f: &mut Frame, event: Option<&Event>, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" 🔍 Detaljerad information ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));

    if let Some(ev) = event {
        let mut lines = Vec::new();

        let act_badge = match ev.activity_type {
            ActivityType::Lecture => Span::styled(format!(" {} FÖRELÄSNING ", ev.activity_type.icon()), Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ActivityType::Exercise => Span::styled(format!(" {} ÖVNING ", ev.activity_type.icon()), Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)),
            ActivityType::Lab => Span::styled(format!(" {} LABORATION ", ev.activity_type.icon()), Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)),
            ActivityType::Exam => Span::styled(format!(" {} TENTAMEN ", ev.activity_type.icon()), Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)),
            ActivityType::Seminar => Span::styled(format!(" {} SEMINARIUM ", ev.activity_type.icon()), Style::default().fg(Color::Black).bg(Color::Blue).add_modifier(Modifier::BOLD)),
            ActivityType::Other(_) => Span::styled(format!(" {} INFO/ÖVRIGT ", ev.activity_type.icon()), Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
        };

        lines.push(Line::from(vec![act_badge]));
        lines.push(Line::raw(""));

        lines.push(Line::from(vec![
            Span::styled("Kurs: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(&ev.course, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("När: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} {} (Vecka {}, {})", ev.day, ev.date, ev.week, ev.week_year), Style::default().fg(Color::White)),
        ]));

        let duration_text = if !ev.duration_display.is_empty() {
            format!("  [Längd: {}]", ev.duration_display)
        } else {
            String::new()
        };
        lines.push(Line::from(vec![
            Span::styled("Tid: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(&ev.time_span, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(duration_text, Style::default().fg(Color::Gray)),
        ]));

        lines.push(Line::raw(""));

        let room_details = ev.room_info_formatted();
        let rooms_text = if room_details.is_empty() {
            "Ingen lokal angiven (Distans/Okänd)".to_string()
        } else {
            room_details.join("\n       ")
        };
        lines.push(Line::from(vec![
            Span::styled("Lokal: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(rooms_text, Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
        ]));

        let sign_text = if ev.signatures.is_empty() {
            "Ingen lärare angiven".to_string()
        } else {
            ev.signatures.join(", ")
        };
        lines.push(Line::from(vec![
            Span::styled("Lärare: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(sign_text, Style::default().fg(Color::LightYellow)),
        ]));

        if !ev.groups.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Grupp: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(ev.groups.join(", "), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            ]));
        }

        if !ev.aids.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Hjälpm: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(&ev.aids, Style::default().fg(Color::Gray)),
            ]));
        }

        lines.push(Line::raw(""));

        lines.push(Line::from(vec![
            Span::styled("Beskrivning / Moment:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(&ev.moment, Style::default().fg(Color::White)),
        ]));

        if !ev.urls.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled("Länkar (Tryck 'w' för att öppna):", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]));
            for u in &ev.urls {
                lines.push(Line::from(vec![
                    Span::styled(format!("  🔗 {}", u), Style::default().fg(Color::LightBlue).add_modifier(Modifier::UNDERLINED)),
                ]));
            }
        }

        if !ev.updated.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled(format!("Senast uppdaterad: {}", ev.updated), Style::default().fg(Color::DarkGray)),
            ]));
        }

        let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        f.render_widget(p, area);
    } else {
        let p = Paragraph::new("Ingen händelse markerad.\nNavigera med piltangenterna eller j/k.").block(block);
        f.render_widget(p, area);
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    if let Some(warn) = &app.warning_message {
        let p = Paragraph::new(Line::from(vec![
            Span::styled(" ⚠️  ", Style::default().fg(Color::Yellow)),
            Span::styled(warn, Style::default().fg(Color::Yellow)),
        ]));
        f.render_widget(p, chunks[0]);
    } else if let Some(status) = app.get_status() {
        let p = Paragraph::new(Line::from(vec![
            Span::styled(" ℹ️  ", Style::default().fg(Color::Green)),
            Span::styled(status, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        f.render_widget(p, chunks[0]);
    } else {
        let p = Paragraph::new(Line::from(vec![
            Span::styled(" [HB Schema TUI] Tryck '?' för alla kortkommandon.", Style::default().fg(Color::DarkGray)),
        ]));
        f.render_widget(p, chunks[0]);
    }

    let shortcuts = Line::from(vec![
        Span::styled("[q]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Avsluta "),
        Span::styled("[Tab]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Vy "),
        Span::styled("[↑/↓ j/k]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Navigera "),
        Span::styled("[←/→ h/l]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Vecka "),
        Span::styled("[/]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Sök "),
        Span::styled("[g]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Grupp "),
        Span::styled("[t]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Idag "),
        Span::styled("[s]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Byt schema "),
        Span::styled("[r]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Uppdatera "),
        Span::styled("[o]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Webbläsare "),
        Span::styled("[?]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Hjälp"),
    ]);

    let footer_p = Paragraph::new(shortcuts).alignment(Alignment::Center);
    f.render_widget(footer_p, chunks[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn render_help_modal(f: &mut Frame) {
    let area = centered_rect(70, 75, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(" 📖 Hjälp & Kortkommandon ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(15, 20, 30)));

    let help_text = vec![
        Line::from(vec![
            Span::styled("NAVIGERING:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  j / k  eller  ↓ / ↑      ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Flytta markering upp / ner i listan"),
        ]),
        Line::from(vec![
            Span::styled("  h / l  eller  ← / →      ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Byt vecka (i Veckovy) eller dag (i Dagvy)"),
        ]),
        Line::from(vec![
            Span::styled("  p / n                    ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Föregående (previous) / Nästa (next) vecka"),
        ]),
        Line::from(vec![
            Span::styled("  t                        ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Hoppa direkt till idag / innevarande vecka"),
        ]),
        Line::from(vec![
            Span::styled("  w                        ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Hoppa direkt till specifikt veckonummer"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("VYER & FLIKAR:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Tab / BackTab            ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Växla vy (Vecka -> Alla händelser -> Dag -> Kurser)"),
        ]),
        Line::from(vec![
            Span::styled("  1 / 2 / 3 / 4            ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Gå direkt till vy 1, 2, 3 eller 4"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("SÖK & FILTER:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  /                        ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Öppna fritextsökning (filtrerar kurs, sal, lärare, text)"),
        ]),
        Line::from(vec![
            Span::styled("  g  eller  f              ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Välj gruppfilter (kryssrutor: markera t.ex. Grupp 1 & Grupp A)"),
        ]),
        Line::from(vec![
            Span::styled("  Esc                      ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Rensa sökning / Stäng meny"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("ÅTGÄRDER:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Enter / d                ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Öppna stor detaljvy för markerad händelse"),
        ]),
        Line::from(vec![
            Span::styled("  s                        ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Växla schema-profil (laddar andra konfigurerade scheman)"),
        ]),
        Line::from(vec![
            Span::styled("  r                        ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Tvinga uppdatering / hämta om schemat från servern"),
        ]),
        Line::from(vec![
            Span::styled("  o                        ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Öppna HB-schemat i extern standardwebbläsare"),
        ]),
        Line::from(vec![
            Span::styled("  w                        ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Öppna webblänk från markerad händelse (om sådan finns)"),
        ]),
        Line::from(vec![
            Span::styled("  q                        ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("Avsluta programmet"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Tryck [Esc] eller [?] för att stänga denna ruta", Style::default().fg(Color::Green).add_modifier(Modifier::ITALIC)),
        ]),
    ];

    let p = Paragraph::new(help_text).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn render_search_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(" 🔍 Sök i schemat ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(20, 25, 35)));

    let text = vec![
        Line::from(vec![
            Span::raw("Skriv sökord (kurs, lokal, lärare, moment, 'grupp 1' etc.):"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(&app.search_input_buffer, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[Enter] Verkställ sökning   [Esc] Avbryt / Rensa", Style::default().fg(Color::Gray)),
        ]),
    ];

    let p = Paragraph::new(text).block(block);
    f.render_widget(p, area);
}

fn render_group_filter_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(65, 75, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(
            " 👥 Välj gruppfilter (kryssa i en eller flera) ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Rgb(20, 25, 35)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Info & Active summary
            Constraint::Min(8),    // List of checkbox items
            Constraint::Length(3), // Action bar / shortcuts
        ])
        .split(inner);

    // Active summary
    let summary_text = if app.selected_groups.is_empty() {
        Line::from(vec![
            Span::styled("Aktiva filter: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Alla grupper (inga filter aktiva - visar alla övningar och laborationer)", Style::default().fg(Color::Green)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Aktiva filter: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(app.selected_groups_display(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" ({} valda)", app.selected_groups.len()), Style::default().fg(Color::Gray)),
        ])
    };

    let intro_p = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Tips: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("Markera t.ex. både Grupp 1 (övningar) och Grupp A (laborationer). Föreläsningar visas alltid."),
        ]),
        summary_text,
    ]);
    f.render_widget(intro_p, chunks[0]);

    let options = app.get_group_options();

    let items: Vec<ListItem> = options.iter().enumerate().map(|(idx, item)| {
        let is_selected = idx == app.selected_group_index;
        let is_checked = app.selected_groups.contains(&item.key);

        let row_style = if is_selected {
            Style::default().fg(Color::White).bg(Color::Rgb(30, 60, 90)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let (box_marker, box_style) = if is_checked {
            ("[✔] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            ("[ ] ", Style::default().fg(Color::DarkGray))
        };

        let shortcut_str = if idx < 9 {
            format!("[{}] ", idx + 1)
        } else {
            "    ".to_string()
        };

        ListItem::new(Line::from(vec![
            Span::styled(box_marker, box_style),
            Span::styled(shortcut_str, Style::default().fg(Color::Yellow)),
            Span::styled(format!("{:<10}", item.title), Style::default().fg(if is_checked { Color::White } else { Color::Gray }).add_modifier(if is_checked { Modifier::BOLD } else { Modifier::empty() })),
            Span::styled(format!(" - {}", item.description), Style::default().fg(Color::DarkGray)),
        ])).style(row_style)
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(list, chunks[1]);

    let help_lines = vec![
        Line::from(vec![
            Span::styled("[Mellanslag]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Kryssa i/ur  "),
            Span::styled("[1-9]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Snabbval  "),
            Span::styled("[c]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Rensa alla  "),
            Span::styled("[a]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Välj alla  "),
            Span::styled("[Enter/Esc]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" Klar"),
        ]),
    ];
    let help_p = Paragraph::new(help_lines).alignment(Alignment::Center);
    f.render_widget(help_p, chunks[2]);
}

fn render_switch_profile_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(" 🔄 Byt schema-profil ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green))
        .style(Style::default().bg(Color::Rgb(20, 25, 35)));

    let profile_keys: Vec<&String> = app.config.profiles.keys().collect();

    let items: Vec<ListItem> = profile_keys.iter().enumerate().map(|(idx, key)| {
        let is_selected = idx == app.selected_profile_index;
        let is_current = *key == &app.active_profile_key;
        let profile = app.config.profiles.get(*key).unwrap();

        let style = if is_selected {
            Style::default().fg(Color::White).bg(Color::Rgb(30, 60, 90)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let marker = if is_current { " ★ " } else { "   " };

        ListItem::new(vec![
            Line::from(vec![
                Span::styled(marker, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}. ", idx + 1), Style::default().fg(Color::Cyan)),
                Span::styled(&profile.name, Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(format!("     Nyckel: [{}]", key), Style::default().fg(Color::Gray)),
            ]),
        ]).style(style)
    }).collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_event_details_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);

    let selected_event = app.get_selected_event();
    render_event_inspector(f, selected_event, area);
}

fn render_goto_week_modal(f: &mut Frame, buffer: &str) {
    let area = centered_rect(40, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(" 📅 Gå till vecka ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(20, 25, 35)));

    let text = vec![
        Line::from(vec![
            Span::raw("Ange veckonummer (t.ex. 36, 40, 5):"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Vecka: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(buffer, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[Enter] Hoppa till vecka   [Esc] Avbryt", Style::default().fg(Color::Gray)),
        ]),
    ];

    let p = Paragraph::new(text).block(block);
    f.render_widget(p, area);
}
