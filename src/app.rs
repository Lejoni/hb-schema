use crate::config::{AppConfig, Profile};
use crate::fetcher::{FetchResult, ScheduleFetcher};
use crate::parser::{ActivityType, Event, Schedule, WeekInfo};
use anyhow::Result;
use chrono::{Datelike, Local};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Week,
    Timeline,
    Day,
    Courses,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    None,
    Help,
    Search,
    GroupFilter,
    SwitchProfile,
    EventDetails,
    GoToWeek(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupOptionItem {
    pub key: String,
    pub title: String,
    pub description: String,
}

pub fn parse_group_filter_string(s: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for part in s.split(',') {
        let trimmed = part.trim().to_lowercase();
        if !trimmed.is_empty() && trimmed != "alla" && trimmed != "all" && trimmed != "*" {
            set.insert(trimmed);
        }
    }
    set
}

pub fn capitalize_group_name(g: &str) -> String {
    let lower = g.trim().to_lowercase();
    if lower.starts_with("grupp ") {
        let rest = &lower["grupp ".len()..];
        if rest.len() == 1 && rest.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
            format!("Grupp {}", rest.to_uppercase())
        } else {
            format!("Grupp {}", rest)
        }
    } else {
        let mut chars = lower.chars();
        match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

pub struct App {
    pub config: AppConfig,
    pub _config_path: PathBuf,
    pub active_profile_key: String,
    pub active_profile: Profile,
    pub fetcher: ScheduleFetcher,
    pub schedule: Option<Schedule>,
    pub fetch_info: Option<String>,
    pub warning_message: Option<String>,

    pub view_mode: ViewMode,
    pub modal: Modal,

    // Navigation state
    pub selected_event_index: usize,
    pub selected_week_index: usize,
    pub selected_day_index: usize,
    pub selected_course_index: usize,
    pub selected_profile_index: usize,
    pub selected_group_index: usize,

    // Filtering & Search
    pub search_query: String,
    pub search_input_buffer: String,
    pub selected_groups: BTreeSet<String>,
    pub activity_filter: Option<ActivityType>,

    // UI feedback
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: AppConfig, config_path: PathBuf, initial_profile: Option<&str>, initial_url: Option<&str>) -> Result<Self> {
        let (profile_key, profile) = if let Some(url) = initial_url {
            let p = Profile {
                name: "Anpassat schema".to_string(),
                url: url.to_string(),
                group_filter: "alla".to_string(),
                description: Some("Länkat via kommandorad".to_string()),
            };
            ("custom".to_string(), p)
        } else {
            let (key, p) = config.get_active_profile(initial_profile)
                .map(|(k, v)| (k.to_string(), v.clone()))
                .unwrap_or_else(|| {
                    let p = Profile {
                        name: "Standard".to_string(),
                        url: "https://schema.hb.se".to_string(),
                        group_filter: "alla".to_string(),
                        description: None,
                    };
                    ("default".to_string(), p)
                });
            (key, p)
        };

        let fetcher = ScheduleFetcher::new(config.cache_ttl_minutes)?;
        let selected_groups = parse_group_filter_string(&profile.group_filter);

        let mut app = Self {
            config,
            _config_path: config_path,
            active_profile_key: profile_key,
            active_profile: profile,
            fetcher,
            schedule: None,
            fetch_info: None,
            warning_message: None,
            view_mode: ViewMode::Week,
            modal: Modal::None,
            selected_event_index: 0,
            selected_week_index: 0,
            selected_day_index: 0,
            selected_course_index: 0,
            selected_profile_index: 0,
            selected_group_index: 0,
            search_query: String::new(),
            search_input_buffer: String::new(),
            selected_groups,
            activity_filter: None,
            status_message: None,
            should_quit: false,
        };

        app.load_schedule(false)?;
        app.jump_to_current_week();
        Ok(app)
    }

    pub fn load_schedule(&mut self, force_refresh: bool) -> Result<()> {
        let result: FetchResult = self.fetcher.fetch(&self.active_profile.url, force_refresh)?;

        let info = if result.from_cache {
            if let Some(age) = result.cache_age {
                let mins = age.as_secs() / 60;
                if mins == 0 {
                    "Cachad (nyss)".to_string()
                } else if mins < 60 {
                    format!("Cachad (för {} min sedan)", mins)
                } else {
                    format!("Cachad (för {}h {}m sedan)", mins / 60, mins % 60)
                }
            } else {
                "Cachad offline".to_string()
            }
        } else {
            "Hämtat live från HB".to_string()
        };

        self.fetch_info = Some(info);
        self.warning_message = result.warning;
        self.schedule = Some(result.schedule);
        self.selected_event_index = 0;
        self.set_status("Schemat laddades utan problem.");
        Ok(())
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn get_status(&self) -> Option<&str> {
        if let Some((msg, time)) = &self.status_message {
            if time.elapsed().as_secs() < 5 {
                return Some(msg.as_str());
            }
        }
        None
    }

    pub fn get_current_week_number() -> u32 {
        let now = Local::now().naive_local();
        now.iso_week().week()
    }

    pub fn jump_to_current_week(&mut self) {
        let current_w = Self::get_current_week_number();
        if let Some(schedule) = &self.schedule {
            if let Some(pos) = schedule.weeks.iter().position(|w| w.week_number >= current_w) {
                self.selected_week_index = pos;
            } else if !schedule.weeks.is_empty() {
                self.selected_week_index = schedule.weeks.len() - 1;
            }
        }
        self.selected_event_index = 0;
    }

    pub fn jump_to_today(&mut self) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        if let Some(schedule) = &self.schedule {
            for (w_idx, w) in schedule.weeks.iter().enumerate() {
                let events = self.get_filtered_events_for_week(w.week_number, w.year);
                if let Some(e_idx) = events.iter().position(|e| e.full_date == today) {
                    self.selected_week_index = w_idx;
                    self.selected_event_index = e_idx;
                    self.set_status("Hoppade till dagens schema.");
                    return;
                }
            }
        }
        self.jump_to_current_week();
        self.set_status("Inga händelser hittades för idag. Visar aktuell vecka.");
    }

    pub fn get_current_week_info(&self) -> Option<&WeekInfo> {
        self.schedule.as_ref().and_then(|s| s.weeks.get(self.selected_week_index))
    }

    pub fn get_filtered_events_for_week(&self, week_num: u32, year: u32) -> Vec<&Event> {
        if let Some(schedule) = &self.schedule {
            schedule.events.iter()
                .filter(|e| e.week == week_num && e.week_year == year)
                .filter(|e| e.matches_filter(&self.search_query))
                .filter(|e| e.matches_groups(&self.selected_groups))
                .filter(|e| {
                    if let Some(ref act) = self.activity_filter {
                        &e.activity_type == act
                    } else {
                        true
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_all_filtered_events(&self) -> Vec<&Event> {
        if let Some(schedule) = &self.schedule {
            schedule.events.iter()
                .filter(|e| e.matches_filter(&self.search_query))
                .filter(|e| e.matches_groups(&self.selected_groups))
                .filter(|e| {
                    if let Some(ref act) = self.activity_filter {
                        &e.activity_type == act
                    } else {
                        true
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn selected_groups_display(&self) -> String {
        if self.selected_groups.is_empty() {
            "Alla".to_string()
        } else {
            self.selected_groups.iter()
                .map(|g| capitalize_group_name(g))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    pub fn set_group_filter(&mut self, filter: &str) {
        self.selected_groups = parse_group_filter_string(filter);
    }

    pub fn get_group_options(&self) -> Vec<GroupOptionItem> {
        let default_options = vec![
            ("grupp 1", "Grupp 1", "Övningar & seminarier"),
            ("grupp 2", "Grupp 2", "Övningar & seminarier"),
            ("grupp 3", "Grupp 3", "Övningar & seminarier"),
            ("grupp 4", "Grupp 4", "Övningar & seminarier"),
            ("grupp 5", "Grupp 5", "Övningar & seminarier"),
            ("grupp 6", "Grupp 6", "Övningar & seminarier"),
            ("grupp a", "Grupp A", "Laborationsgrupp A"),
            ("grupp b", "Grupp B", "Laborationsgrupp B"),
            ("grupp c", "Grupp C", "Laborationsgrupp C"),
            ("grupp d", "Grupp D", "Laborationsgrupp D"),
            ("grupp e", "Grupp E", "Laborationsgrupp E"),
            ("grupp f", "Grupp F", "Laborationsgrupp F"),
            ("grupp g", "Grupp G", "Laborationsgrupp G"),
            ("grupp h", "Grupp H", "Laborationsgrupp H"),
        ];

        let mut result: Vec<GroupOptionItem> = default_options
            .into_iter()
            .map(|(k, t, d)| GroupOptionItem {
                key: k.to_string(),
                title: t.to_string(),
                description: d.to_string(),
            })
            .collect();

        if let Some(schedule) = &self.schedule {
            for ev in &schedule.events {
                for g in &ev.groups {
                    let g_clean = g.trim().to_lowercase();
                    if g_clean.contains("och") || g_clean.contains('/') || g_clean == "alla" {
                        continue;
                    }
                    if !result.iter().any(|item| item.key == g_clean) {
                        let title = capitalize_group_name(&g_clean);
                        result.push(GroupOptionItem {
                            key: g_clean,
                            title,
                            description: "Från schemat".to_string(),
                        });
                    }
                }
            }
        }

        result
    }

    pub fn get_current_view_events(&self) -> Vec<&Event> {
        match self.view_mode {
            ViewMode::Week => {
                if let Some(w) = self.get_current_week_info() {
                    self.get_filtered_events_for_week(w.week_number, w.year)
                } else {
                    Vec::new()
                }
            }
            ViewMode::Timeline => self.get_all_filtered_events(),
            ViewMode::Day => {
                let all_events = self.get_all_filtered_events();
                let days = self.get_unique_days();
                if let Some(target_day) = days.get(self.selected_day_index) {
                    all_events.into_iter().filter(|e| &e.full_date == target_day).collect()
                } else {
                    Vec::new()
                }
            }
            ViewMode::Courses => Vec::new(),
        }
    }

    pub fn get_unique_days(&self) -> Vec<String> {
        let events = self.get_all_filtered_events();
        let mut days: Vec<String> = events.iter().map(|e| e.full_date.clone()).collect();
        days.dedup();
        days
    }

    pub fn get_selected_event(&self) -> Option<&Event> {
        let events = self.get_current_view_events();
        events.get(self.selected_event_index).copied()
    }

    pub fn next_event(&mut self) {
        let count = self.get_current_view_events().len();
        if count > 0 {
            if self.selected_event_index + 1 < count {
                self.selected_event_index += 1;
            } else {
                self.selected_event_index = 0;
            }
        }
    }

    pub fn prev_event(&mut self) {
        let count = self.get_current_view_events().len();
        if count > 0 {
            if self.selected_event_index > 0 {
                self.selected_event_index -= 1;
            } else {
                self.selected_event_index = count - 1;
            }
        }
    }

    pub fn next_week(&mut self) {
        if let Some(schedule) = &self.schedule {
            if self.selected_week_index + 1 < schedule.weeks.len() {
                self.selected_week_index += 1;
                self.selected_event_index = 0;
            }
        }
    }

    pub fn prev_week(&mut self) {
        if self.selected_week_index > 0 {
            self.selected_week_index -= 1;
            self.selected_event_index = 0;
        }
    }

    pub fn next_day(&mut self) {
        let days = self.get_unique_days();
        if self.selected_day_index + 1 < days.len() {
            self.selected_day_index += 1;
            self.selected_event_index = 0;
        }
    }

    pub fn prev_day(&mut self) {
        if self.selected_day_index > 0 {
            self.selected_day_index -= 1;
            self.selected_event_index = 0;
        }
    }

    pub fn next_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Week => ViewMode::Timeline,
            ViewMode::Timeline => ViewMode::Day,
            ViewMode::Day => ViewMode::Courses,
            ViewMode::Courses => ViewMode::Week,
        };
        self.selected_event_index = 0;
    }

    pub fn switch_to_profile(&mut self, profile_key: String) -> Result<()> {
        if let Some(p) = self.config.profiles.get(&profile_key) {
            self.active_profile_key = profile_key;
            self.active_profile = p.clone();
            self.selected_groups = parse_group_filter_string(&p.group_filter);
            self.load_schedule(false)?;
            self.jump_to_current_week();
            self.set_status(format!("Växlade till schema: {}", self.active_profile.name));
        }
        Ok(())
    }

    pub fn open_browser_schedule(&self) {
        if let Err(e) = open::that(&self.active_profile.url) {
            eprintln!("Kunde inte öppna webbläsare: {}", e);
        }
    }

    pub fn open_selected_event_url(&mut self) {
        if let Some(event) = self.get_selected_event() {
            if let Some(first_url) = event.urls.first() {
                if let Ok(_) = open::that(first_url) {
                    self.set_status(format!("Öppnade länk i webbläsare: {}", first_url));
                    return;
                }
            }
        }
        self.set_status("Ingen webblänk hittades i den markerade händelsen.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_group_filter_string() {
        let set = parse_group_filter_string("grupp 1, grupp a");
        assert!(set.contains("grupp 1"));
        assert!(set.contains("grupp a"));
        assert_eq!(set.len(), 2);

        let alla_set = parse_group_filter_string("alla");
        assert!(alla_set.is_empty());

        let empty_set = parse_group_filter_string("");
        assert!(empty_set.is_empty());
    }

    #[test]
    fn test_capitalize_group_name() {
        assert_eq!(capitalize_group_name("grupp 1"), "Grupp 1");
        assert_eq!(capitalize_group_name("grupp a"), "Grupp A");
        assert_eq!(capitalize_group_name("grupp b"), "Grupp B");
        assert_eq!(capitalize_group_name("pgrp1"), "Pgrp1");
    }
}
