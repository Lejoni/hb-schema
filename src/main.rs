mod app;
mod cli;
mod config;
mod fetcher;
mod parser;
mod ui;

use anyhow::Result;
use app::{App, Modal, ViewMode};
use clap::Parser;
use cli::CliArgs;
use config::AppConfig;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

fn main() -> Result<()> {
    let args = CliArgs::parse();

    // Load or create configuration
    let (config, config_path) = AppConfig::load_or_create(args.config.as_deref())?;

    // Handle CLI non-interactive actions if requested
    if args.today || args.week.is_some() || args.export_ical.is_some() {
        return run_cli_action(args, config, config_path);
    }

    // Initialize App
    let mut app = App::new(
        config,
        config_path,
        args.profile.as_deref(),
        args.url.as_deref(),
    )?;

    if let Some(grp) = args.group.as_deref() {
        app.set_group_filter(grp);
    }
    if args.refresh {
        let _ = app.load_schedule(true);
    }

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Fel vid körning av schema-TUI: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let CEvent::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key_event(app, key.code, key.modifiers)?;
                }
            }
        }
    }
    Ok(())
}

fn handle_key_event(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> Result<()> {
    match &mut app.modal {
        Modal::Search => {
            match code {
                KeyCode::Enter => {
                    app.search_query = app.search_input_buffer.clone();
                    app.modal = Modal::None;
                    app.selected_event_index = 0;
                    let msg = if app.search_query.is_empty() {
                        "Sökning rensad.".to_string()
                    } else {
                        format!("Filtrerar på \"{}\"", app.search_query)
                    };
                    app.set_status(msg);
                }
                KeyCode::Esc => {
                    app.modal = Modal::None;
                }
                KeyCode::Backspace => {
                    app.search_input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    app.search_input_buffer.push(c);
                }
                _ => {}
            }
        }
        Modal::GoToWeek(buf) => {
            match code {
                KeyCode::Enter => {
                    if let Ok(target_w) = buf.trim().parse::<u32>() {
                        if let Some(schedule) = &app.schedule {
                            if let Some(pos) = schedule.weeks.iter().position(|w| w.week_number == target_w) {
                                app.selected_week_index = pos;
                                app.selected_event_index = 0;
                                app.set_status(format!("Hoppade till vecka {}", target_w));
                            } else {
                                app.set_status(format!("Vecka {} hittades inte i schemat.", target_w));
                            }
                        }
                    }
                    app.modal = Modal::None;
                }
                KeyCode::Esc => {
                    app.modal = Modal::None;
                }
                KeyCode::Backspace => {
                    buf.pop();
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    buf.push(c);
                }
                _ => {}
            }
        }
        Modal::GroupFilter => {
            let options = app.get_group_options();
            match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('g') | KeyCode::Char('f') | KeyCode::Enter => {
                    let msg = if app.selected_groups.is_empty() {
                        "Visar alla grupper.".to_string()
                    } else {
                        format!("Aktiva gruppfilter: {}", app.selected_groups_display())
                    };
                    app.set_status(msg);
                    app.modal = Modal::None;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.selected_group_index > 0 {
                        app.selected_group_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.selected_group_index + 1 < options.len() {
                        app.selected_group_index += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(item) = options.get(app.selected_group_index) {
                        if app.selected_groups.contains(&item.key) {
                            app.selected_groups.remove(&item.key);
                        } else {
                            app.selected_groups.insert(item.key.clone());
                        }
                        app.selected_event_index = 0;
                    }
                }
                KeyCode::Char('c') => {
                    app.selected_groups.clear();
                    app.selected_event_index = 0;
                    app.set_status("Alla gruppfilter rensade (visar alla grupper).");
                }
                KeyCode::Char('a') => {
                    if app.selected_groups.len() >= options.len() {
                        app.selected_groups.clear();
                    } else {
                        for item in &options {
                            app.selected_groups.insert(item.key.clone());
                        }
                    }
                    app.selected_event_index = 0;
                }
                KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                    if let Some(digit) = c.to_digit(10) {
                        let idx = (digit as usize).saturating_sub(1);
                        if let Some(item) = options.get(idx) {
                            if app.selected_groups.contains(&item.key) {
                                app.selected_groups.remove(&item.key);
                            } else {
                                app.selected_groups.insert(item.key.clone());
                            }
                            app.selected_event_index = 0;
                        }
                    }
                }
                _ => {}
            }
        }
        Modal::SwitchProfile => {
            let profile_keys: Vec<String> = app.config.profiles.keys().cloned().collect();
            match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('s') => {
                    app.modal = Modal::None;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.selected_profile_index > 0 {
                        app.selected_profile_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.selected_profile_index + 1 < profile_keys.len() {
                        app.selected_profile_index += 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(key) = profile_keys.get(app.selected_profile_index) {
                        app.switch_to_profile(key.clone())?;
                    }
                    app.modal = Modal::None;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    if let Some(digit) = c.to_digit(10) {
                        let idx = (digit as usize).saturating_sub(1);
                        if let Some(key) = profile_keys.get(idx) {
                            app.switch_to_profile(key.clone())?;
                            app.modal = Modal::None;
                        }
                    }
                }
                _ => {}
            }
        }
        Modal::Help => {
            match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Enter => {
                    app.modal = Modal::None;
                }
                _ => {}
            }
        }
        Modal::EventDetails => {
            match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char('d') => {
                    app.modal = Modal::None;
                }
                KeyCode::Char('w') | KeyCode::Char('u') => {
                    app.open_selected_event_url();
                }
                KeyCode::Char('o') => {
                    app.open_browser_schedule();
                }
                _ => {}
            }
        }
        Modal::None => {
            match code {
                KeyCode::Char('q') => {
                    app.should_quit = true;
                }
                KeyCode::Tab => {
                    app.next_mode();
                }
                KeyCode::BackTab => {
                    app.view_mode = match app.view_mode {
                        ViewMode::Week => ViewMode::Courses,
                        ViewMode::Timeline => ViewMode::Week,
                        ViewMode::Day => ViewMode::Timeline,
                        ViewMode::Courses => ViewMode::Day,
                    };
                    app.selected_event_index = 0;
                }
                KeyCode::Char('1') => {
                    app.view_mode = ViewMode::Week;
                    app.selected_event_index = 0;
                }
                KeyCode::Char('2') => {
                    app.view_mode = ViewMode::Timeline;
                    app.selected_event_index = 0;
                }
                KeyCode::Char('3') => {
                    app.view_mode = ViewMode::Day;
                    app.selected_event_index = 0;
                }
                KeyCode::Char('4') => {
                    app.view_mode = ViewMode::Courses;
                    app.selected_event_index = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.view_mode == ViewMode::Courses {
                        let count = app.config.profiles.len().max(5);
                        if app.selected_course_index + 1 < count {
                            app.selected_course_index += 1;
                        }
                    } else {
                        app.next_event();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.view_mode == ViewMode::Courses {
                        if app.selected_course_index > 0 {
                            app.selected_course_index -= 1;
                        }
                    } else {
                        app.prev_event();
                    }
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('n') => {
                    if app.view_mode == ViewMode::Day {
                        app.next_day();
                    } else {
                        app.next_week();
                    }
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('p') => {
                    if app.view_mode == ViewMode::Day {
                        app.prev_day();
                    } else {
                        app.prev_week();
                    }
                }
                KeyCode::Char('t') => {
                    app.jump_to_today();
                }
                KeyCode::Char('w') => {
                    app.modal = Modal::GoToWeek(String::new());
                }
                KeyCode::Char('/') => {
                    app.search_input_buffer = app.search_query.clone();
                    app.modal = Modal::Search;
                }
                KeyCode::Char('g') | KeyCode::Char('f') => {
                    app.modal = Modal::GroupFilter;
                }
                KeyCode::Esc => {
                    if !app.search_query.is_empty() {
                        app.search_query.clear();
                        app.set_status("Sökfilter rensat.");
                    }
                }
                KeyCode::Char('s') => {
                    app.modal = Modal::SwitchProfile;
                }
                KeyCode::Char('r') => {
                    app.set_status("Hämtar nytt schema från servern...");
                    if let Err(e) = app.load_schedule(true) {
                        app.set_status(format!("Kunde inte uppdatera: {}", e));
                    }
                }
                KeyCode::Enter | KeyCode::Char('d') | KeyCode::Char(' ') => {
                    app.modal = Modal::EventDetails;
                }
                KeyCode::Char('o') => {
                    app.open_browser_schedule();
                    app.set_status("Öppnade schemat i webbläsaren.");
                }
                KeyCode::Char('u') => {
                    app.open_selected_event_url();
                }
                KeyCode::Char('?') | KeyCode::F(1) => {
                    app.modal = Modal::Help;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn run_cli_action(args: CliArgs, config: AppConfig, _config_path: std::path::PathBuf) -> Result<()> {
    let (_, profile) = config.get_active_profile(args.profile.as_deref()).unwrap();
    let url = args.url.as_deref().unwrap_or(&profile.url);
    let fetcher = fetcher::ScheduleFetcher::new(config.cache_ttl_minutes)?;
    let result = fetcher.fetch(url, args.refresh)?;
    let schedule = result.schedule;

    if let Some(export_path) = args.export_ical {
        if let Some(ref ical_url) = schedule.metadata.ical_url {
            println!("Laddar ner iCal-kalender till {:?}", export_path);
            fetcher.download_ical(ical_url, &export_path)?;
            println!("Klart! Kalender sparad.");
            return Ok(());
        } else {
            eprintln!("Ingen iCal-länk hittades i schemat.");
            return Ok(());
        }
    }

    let cli_groups = args.group.as_deref()
        .map(app::parse_group_filter_string)
        .unwrap_or_else(|| app::parse_group_filter_string(&profile.group_filter));

    if args.today {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let events: Vec<_> = schedule.events.iter()
            .filter(|e| e.full_date == today)
            .filter(|e| e.matches_groups(&cli_groups))
            .collect();
        println!("==================================================");
        println!("  HB SCHEMA - IDAG ({})", today);
        println!("  Program: {}", schedule.metadata.program_info.trim());
        if !cli_groups.is_empty() {
            println!("  Gruppfilter: {}", cli_groups.iter().map(|g| app::capitalize_group_name(g)).collect::<Vec<_>>().join(", "));
        }
        println!("==================================================");
        if events.is_empty() {
            println!("Inga schemalagda aktiviteter idag.");
        } else {
            for (i, ev) in events.iter().enumerate() {
                println!("{}. {} [{}] - {}", i + 1, ev.time_span, ev.activity_type.display_name(), ev.course);
                if !ev.rooms.is_empty() {
                    println!("   Lokal: {}", ev.rooms.join(", "));
                }
                if !ev.signatures.is_empty() {
                    println!("   Lärare: {}", ev.signatures.join(", "));
                }
                println!("   Moment: {}", ev.moment);
                println!();
            }
        }
        return Ok(());
    }

    if let Some(target_week_opt) = args.week {
        let target_week = target_week_opt.unwrap_or_else(|| App::get_current_week_number());
        let events: Vec<_> = schedule.events.iter()
            .filter(|e| e.week == target_week)
            .filter(|e| e.matches_groups(&cli_groups))
            .collect();
        println!("==================================================");
        println!("  HB SCHEMA - VECKA {}", target_week);
        println!("  Program: {}", schedule.metadata.program_info.trim());
        if !cli_groups.is_empty() {
            println!("  Gruppfilter: {}", cli_groups.iter().map(|g| app::capitalize_group_name(g)).collect::<Vec<_>>().join(", "));
        }
        println!("==================================================");
        if events.is_empty() {
            println!("Inga aktiviteter hittades för vecka {}.", target_week);
        } else {
            for ev in events {
                println!("[{} {}] {} | {:<12} | {:<25} | Sal: {:<10} | {}",
                    ev.day, ev.date, ev.time_span, ev.activity_type.display_name(), ev.course_code, ev.rooms.join(", "), ev.moment);
            }
        }
        return Ok(());
    }

    Ok(())
}
