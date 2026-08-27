use anyhow::Result;
use chrono::NaiveTime;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityType {
    Lecture,     // Föreläsning
    Exercise,    // Övning / Lektion / Räknestuga
    Lab,         // Laboration
    Exam,        // Tentamen / Dugga / Prov
    Seminar,     // Seminarium
    Other(String),
}

impl ActivityType {
    pub fn from_moment(moment: &str) -> Self {
        let m_lower = moment.to_lowercase();
        if m_lower.contains("föreläs") || m_lower.contains("forelas") || m_lower.contains("intro") || m_lower.contains("uppstart") {
            ActivityType::Lecture
        } else if m_lower.contains("övn") || m_lower.contains("ovn") || m_lower.contains("räknestuga") || m_lower.contains("handledning") || m_lower.contains("workshop") {
            ActivityType::Exercise
        } else if m_lower.contains("lab") {
            ActivityType::Lab
        } else if m_lower.contains("tent") || m_lower.contains("dugga") || m_lower.contains("prov") || m_lower.contains("examination") {
            ActivityType::Exam
        } else if m_lower.contains("seminar") {
            ActivityType::Seminar
        } else {
            ActivityType::Other(moment.to_string())
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            ActivityType::Lecture => "Föreläsning",
            ActivityType::Exercise => "Övning",
            ActivityType::Lab => "Laboration",
            ActivityType::Exam => "Tentamen",
            ActivityType::Seminar => "Seminarium",
            ActivityType::Other(_) => "Övrigt",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            ActivityType::Lecture => "🎓",
            ActivityType::Exercise => "✏️",
            ActivityType::Lab => "🔬",
            ActivityType::Exam => "📝",
            ActivityType::Seminar => "💬",
            ActivityType::Other(_) => "📌",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub week: u32,
    pub week_year: u32,
    pub day: String,           // e.g. "Mån", "Tis"
    pub date: String,          // e.g. "31 Aug"
    pub full_date: String,     // e.g. "2026-08-31"
    pub time_span: String,     // e.g. "08:15-12:00"
    pub start_time: String,    // e.g. "08:15"
    pub end_time: String,      // e.g. "12:00"
    pub duration_display: String, // e.g. "3h 45m"
    pub course: String,        // e.g. "Matematik Bas 1, 7.5 FUPO 50% HT26"
    pub course_code: String,   // e.g. "40S02A" or parsed short name
    pub signatures: Vec<String>, // e.g. ["FAGO", "ULM"]
    pub rooms: Vec<String>,      // e.g. ["J517", "M404"]
    pub aids: String,            // e.g. "Hjälpmedel"
    pub moment: String,          // e.g. "Föreläsning Uppstart för programmet..."
    pub activity_type: ActivityType,
    pub groups: Vec<String>,     // e.g. ["grupp 1", "grupp A"]
    pub urls: Vec<String>,       // e.g. ["https://studentkareniboras.se/vtschema/"]
    pub updated: String,         // e.g. "2026-07-02"
}

impl Event {
    pub fn room_info_formatted(&self) -> Vec<String> {
        self.rooms.iter().map(|r| {
            let desc = get_building_description(r);
            if desc.is_empty() {
                r.clone()
            } else {
                format!("{} ({})", r, desc)
            }
        }).collect()
    }

    pub fn matches_filter(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.course.to_lowercase().contains(&q)
            || self.moment.to_lowercase().contains(&q)
            || self.day.to_lowercase().contains(&q)
            || self.date.to_lowercase().contains(&q)
            || self.time_span.contains(&q)
            || self.activity_type.display_name().to_lowercase().contains(&q)
            || self.rooms.iter().any(|r| r.to_lowercase().contains(&q))
            || self.signatures.iter().any(|s| s.to_lowercase().contains(&q))
            || self.groups.iter().any(|g| g.to_lowercase().contains(&q))
    }

    #[allow(dead_code)]
    pub fn matches_group(&self, group_filter: &str) -> bool {
        let set = [group_filter.to_string()];
        self.matches_groups(&set)
    }

    pub fn matches_groups<I, S>(&self, group_filters: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut has_filters = false;
        let mut filters_list = Vec::new();

        for item in group_filters {
            let gf = item.as_ref().trim().to_lowercase();
            if !gf.is_empty() && gf != "alla" && gf != "all" && gf != "*" {
                has_filters = true;
                filters_list.push(gf);
            }
        }

        if !has_filters {
            return true;
        }

        // If event doesn't specify a group (e.g. general lecture), all groups should attend!
        if self.groups.is_empty() {
            return true;
        }

        // Check if event matches ANY of the target groups
        for gf in &filters_list {
            for g in &self.groups {
                let g_lower = g.to_lowercase();
                if g_lower.contains(gf) || gf.contains(&g_lower) {
                    return true;
                }
                // Check specific numbers / letters like "1" or "a"
                if gf.len() == 1 {
                    if g_lower.contains(&format!("grupp {}", gf)) || g_lower.contains(&format!("grupp {}", gf.to_uppercase())) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekInfo {
    pub week_number: u32,
    pub year: u32,
    pub date_span: String,
    pub event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleMetadata {
    pub program_info: String,
    pub date_range: String,
    pub printed_at: String,
    pub ical_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub metadata: ScheduleMetadata,
    pub events: Vec<Event>,
    pub weeks: Vec<WeekInfo>,
}

impl Schedule {
    pub fn parse_html(html_str: &str, base_url: Option<&str>) -> Result<Self> {
        let document = Html::parse_document(html_str);

        let mut program_info = String::new();
        let mut date_range = String::new();
        let mut printed_at = String::new();
        let mut ical_url = None;

        let td_selector = Selector::parse("td").unwrap();
        let a_selector = Selector::parse("a").unwrap();

        for element in document.select(&td_selector) {
            let text = element.text().collect::<Vec<_>>().join(" ");
            let trimmed = text.trim();
            if trimmed.starts_with("Datum:") {
                date_range = trimmed.replace("Datum:", "").trim().to_string();
            } else if trimmed.starts_with("Utskrivet:") {
                printed_at = trimmed.replace("Utskrivet:", "").trim().to_string();
            }
        }

        let big2_selector = Selector::parse("td.big2").unwrap();
        if let Some(big2) = document.select(&big2_selector).next() {
            let p_text = big2.text().collect::<Vec<_>>().join(" ");
            let clean = p_text.replace("Program:", "").trim().to_string();
            if !clean.is_empty() {
                program_info = clean;
            }
        }

        // iCal link
        for a in document.select(&a_selector) {
            if let Some(href) = a.value().attr("href") {
                if href.contains("SchemaICAL.ics") || href.contains(".ics") {
                    if href.starts_with("http") {
                        ical_url = Some(href.to_string());
                    } else if let Some(base) = base_url {
                        if let Ok(base_parsed) = url::Url::parse(base) {
                            if let Ok(joined) = base_parsed.join(href) {
                                ical_url = Some(joined.to_string());
                            }
                        }
                    } else {
                        ical_url = Some(format!("https://schema.hb.se{}", href));
                    }
                }
            }
        }

        let tr_selector = Selector::parse("tr").unwrap();
        let mut events = Vec::new();

        let mut current_week = 0u32;
        let mut current_year = 2026u32;
        let mut current_day = String::new();
        let mut current_date = String::new();

        for tr in document.select(&tr_selector) {
            let tr_text = tr.text().collect::<Vec<_>>().join(" ");
            if let Some(class_attr) = tr.value().attr("class") {
                if class_attr.contains("data-white") || class_attr.contains("data-grey") {
                    let tds: Vec<_> = tr.select(&td_selector).collect();
                    if tds.len() >= 9 {
                        let day_raw = tds[1].text().collect::<Vec<_>>().join(" ").trim().replace("&nbsp;", "").replace('\u{a0}', "");
                        let date_raw = tds[2].text().collect::<Vec<_>>().join(" ").trim().replace("&nbsp;", "").replace('\u{a0}', "");
                        let time_raw = tds[3].text().collect::<Vec<_>>().join(" ").trim().replace("&nbsp;", "").replace('\u{a0}', "");
                        let course_raw = tds[4].text().collect::<Vec<_>>().join(" ").trim().to_string();
                        let sign_raw = tds[5].text().collect::<Vec<_>>().join(" ").trim().replace("&nbsp;", "").replace('\u{a0}', "");
                        let room_raw = tds[6].text().collect::<Vec<_>>().join(" ").trim().replace("&nbsp;", "").replace('\u{a0}', "");
                        let aids_raw = if tds.len() > 7 { tds[7].text().collect::<Vec<_>>().join(" ").trim().replace("&nbsp;", "").replace('\u{a0}', "") } else { String::new() };
                        let moment_raw = if tds.len() > 8 { tds[8].text().collect::<Vec<_>>().join(" ").trim().replace("&nbsp;", "").replace('\u{a0}', "") } else { String::new() };
                        let updated_raw = if tds.len() > 9 { tds[9].text().collect::<Vec<_>>().join(" ").trim().replace("&nbsp;", "").replace('\u{a0}', "") } else { String::new() };

                        if !day_raw.is_empty() {
                            current_day = day_raw;
                        }
                        if !date_raw.is_empty() {
                            current_date = date_raw;
                        }

                        let mut start_time = String::new();
                        let mut end_time = String::new();
                        if let Some((s, e)) = time_raw.split_once('-') {
                            start_time = s.trim().to_string();
                            end_time = e.trim().to_string();
                        } else if !time_raw.is_empty() {
                            start_time = time_raw.clone();
                        }

                        let duration_display = compute_duration(&start_time, &end_time);

                        let signatures: Vec<String> = sign_raw
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();

                        let rooms: Vec<String> = room_raw
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();

                        let course_code = extract_course_code(&course_raw);
                        let activity_type = ActivityType::from_moment(&moment_raw);
                        let groups = extract_groups(&moment_raw);
                        let urls = extract_urls(&moment_raw);
                        let full_date = resolve_full_date(&current_date, current_year);

                        events.push(Event {
                            week: current_week,
                            week_year: current_year,
                            day: current_day.clone(),
                            date: current_date.clone(),
                            full_date,
                            time_span: time_raw,
                            start_time,
                            end_time,
                            duration_display,
                            course: course_raw,
                            course_code,
                            signatures,
                            rooms,
                            aids: aids_raw,
                            moment: moment_raw,
                            activity_type,
                            groups,
                            urls,
                            updated: updated_raw,
                        });
                    }
                }
            } else {
                let text = tr_text.trim();
                if text.contains("Vecka") {
                    if let Some(idx) = text.find("Vecka") {
                        let sub = &text[idx..];
                        let parts: Vec<&str> = sub.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(w) = parts[1].trim_matches(',').parse::<u32>() {
                                current_week = w;
                            }
                        }
                        if parts.len() >= 3 {
                            if let Ok(y) = parts[2].trim().parse::<u32>() {
                                current_year = y;
                            }
                        }
                    }
                }
            }
        }

        // Build list of unique weeks with summary
        let mut weeks: Vec<WeekInfo> = Vec::new();
        for ev in &events {
            if let Some(w) = weeks.iter_mut().find(|w| w.week_number == ev.week && w.year == ev.week_year) {
                w.event_count += 1;
            } else {
                weeks.push(WeekInfo {
                    week_number: ev.week,
                    year: ev.week_year,
                    date_span: ev.date.clone(),
                    event_count: 1,
                });
            }
        }

        Ok(Schedule {
            metadata: ScheduleMetadata {
                program_info,
                date_range,
                printed_at,
                ical_url,
            },
            events,
            weeks,
        })
    }
}

fn compute_duration(start: &str, end: &str) -> String {
    if let (Ok(s), Ok(e)) = (
        NaiveTime::parse_from_str(start, "%H:%M"),
        NaiveTime::parse_from_str(end, "%H:%M"),
    ) {
        let diff = e.signed_duration_since(s);
        let mins = diff.num_minutes();
        if mins > 0 {
            let h = mins / 60;
            let m = mins % 60;
            if h > 0 && m > 0 {
                return format!("{}h {}m", h, m);
            } else if h > 0 {
                return format!("{}h", h);
            } else {
                return format!("{}m", m);
            }
        }
    }
    String::new()
}

pub fn get_building_description(room: &str) -> String {
    let r = room.trim().to_uppercase();
    if r.is_empty() {
        return String::new();
    }
    let first_char = r.chars().next().unwrap_or(' ');
    let floor_str = r.chars().nth(1).filter(|c| c.is_ascii_digit()).map(|c| format!("Plan {}", c)).unwrap_or_default();

    let building = match first_char {
        'A' => "Hus A (Balder)",
        'B' => "Hus B (Balder)",
        'C' => "Hus C (Balder)",
        'D' => "Hus D (Balder)",
        'E' => "Hus E (Balder)",
        'J' => "Hus J (Balder)",
        'M' => "Hus M (Sandgärdet)",
        'S' => "Hus S (Sandgärdet)",
        'T' => "Textile Fashion Center",
        'K' => "Kårhuset",
        _ => {
            if r.contains("ZOOM") || r.contains("ONLINE") || r.contains("DISTANS") {
                return "Distans / Zoom".to_string();
            }
            ""
        }
    };

    if building.is_empty() {
        String::new()
    } else if !floor_str.is_empty() {
        format!("{}, {}", building, floor_str)
    } else {
        building.to_string()
    }
}

fn extract_course_code(course: &str) -> String {
    if let Some((name, _)) = course.split_once(',') {
        name.trim().to_string()
    } else {
        course.trim().to_string()
    }
}

fn extract_groups(moment: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let m_lower = moment.to_lowercase();

    // Look for patterns like "grupp 1", "grupp 2", "grupp A och B", etc.
    let patterns = [
        "grupp 1", "grupp 2", "grupp 3", "grupp 4", "grupp 5", "grupp 6",
        "grupp 7", "grupp 8", "grupp 9", "grupp 10", "grupp 11", "grupp 12",
        "grupp a", "grupp b", "grupp c", "grupp d", "grupp e", "grupp f", "grupp g", "grupp h",
        "grupp i", "grupp j", "grupp k", "grupp l",
        "grupp a och b", "grupp c och d", "grupp e och f", "grupp g och h",
        "grupp a/b", "grupp c/d", "grupp e/f", "grupp g/h",
        "grupp 1 och 2", "grupp 3 och 4", "grupp 5 och 6",
        "grupp 1/2", "grupp 3/4", "grupp 5/6",
        "pgrp1", "pgrp2", "pgrp3",
    ];

    for pat in patterns {
        if m_lower.contains(pat) {
            groups.push(pat.to_string());
        }
    }

    groups
}

fn extract_urls(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for word in text.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            let clean = word.trim_end_matches(&[',', '.', ';', ')', '>', ']'][..]);
            urls.push(clean.to_string());
        }
    }
    urls
}

fn resolve_full_date(date_str: &str, year: u32) -> String {
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() >= 2 {
        if let Ok(day) = parts[0].parse::<u32>() {
            let month = match parts[1].to_lowercase().as_str() {
                "jan" => 1,
                "feb" => 2,
                "mar" | "mår" => 3,
                "apr" => 4,
                "maj" | "may" => 5,
                "jun" => 6,
                "jul" => 7,
                "aug" => 8,
                "sep" => 9,
                "okt" | "oct" => 10,
                "nov" => 11,
                "dec" => 12,
                _ => 0,
            };
            if month > 0 {
                return format!("{:04}-{:02}-{:02}", year, month, day);
            }
        }
    }
    date_str.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_type_detection() {
        assert_eq!(ActivityType::from_moment("Föreläsning 40S01A"), ActivityType::Lecture);
        assert_eq!(ActivityType::from_moment("Övning grupp 1"), ActivityType::Exercise);
        assert_eq!(ActivityType::from_moment("Laboration 1 grupp A och B"), ActivityType::Lab);
        assert_eq!(ActivityType::from_moment("Tentamen Matematik Bas 1"), ActivityType::Exam);
    }

    #[test]
    fn test_building_info() {
        assert_eq!(get_building_description("M404"), "Hus M (Sandgärdet), Plan 4");
        assert_eq!(get_building_description("J517"), "Hus J (Balder), Plan 5");
        assert_eq!(get_building_description("D407"), "Hus D (Balder), Plan 4");
    }

    #[test]
    fn test_duration_calc() {
        assert_eq!(compute_duration("08:15", "12:00"), "3h 45m");
        assert_eq!(compute_duration("13:15", "15:00"), "1h 45m");
    }

    #[test]
    fn test_matches_multiple_groups() {
        let lecture = Event {
            week: 36,
            week_year: 2026,
            day: "Mån".to_string(),
            date: "31 Aug".to_string(),
            full_date: "2026-08-31".to_string(),
            time_span: "08:15-10:00".to_string(),
            start_time: "08:15".to_string(),
            end_time: "10:00".to_string(),
            duration_display: "1h 45m".to_string(),
            course: "Matematik".to_string(),
            course_code: "40S02A".to_string(),
            signatures: vec![],
            rooms: vec![],
            aids: "".to_string(),
            moment: "Föreläsning Introduktion".to_string(),
            activity_type: ActivityType::Lecture,
            groups: vec![],
            urls: vec![],
            updated: "".to_string(),
        };

        let exercise_g1 = Event {
            week: 36,
            week_year: 2026,
            day: "Mån".to_string(),
            date: "31 Aug".to_string(),
            full_date: "2026-08-31".to_string(),
            time_span: "10:15-12:00".to_string(),
            start_time: "10:15".to_string(),
            end_time: "12:00".to_string(),
            duration_display: "1h 45m".to_string(),
            course: "Matematik".to_string(),
            course_code: "40S02A".to_string(),
            signatures: vec![],
            rooms: vec![],
            aids: "".to_string(),
            moment: "Övning Grupp 1".to_string(),
            activity_type: ActivityType::Exercise,
            groups: vec!["grupp 1".to_string()],
            urls: vec![],
            updated: "".to_string(),
        };

        let exercise_g2 = Event {
            week: 36,
            week_year: 2026,
            day: "Mån".to_string(),
            date: "31 Aug".to_string(),
            full_date: "2026-08-31".to_string(),
            time_span: "10:15-12:00".to_string(),
            start_time: "10:15".to_string(),
            end_time: "12:00".to_string(),
            duration_display: "1h 45m".to_string(),
            course: "Matematik".to_string(),
            course_code: "40S02A".to_string(),
            signatures: vec![],
            rooms: vec![],
            aids: "".to_string(),
            moment: "Övning Grupp 2".to_string(),
            activity_type: ActivityType::Exercise,
            groups: vec!["grupp 2".to_string()],
            urls: vec![],
            updated: "".to_string(),
        };

        let lab_ab = Event {
            week: 36,
            week_year: 2026,
            day: "Tis".to_string(),
            date: "1 Sep".to_string(),
            full_date: "2026-09-01".to_string(),
            time_span: "13:15-17:00".to_string(),
            start_time: "13:15".to_string(),
            end_time: "17:00".to_string(),
            duration_display: "3h 45m".to_string(),
            course: "Kemi".to_string(),
            course_code: "40S01A".to_string(),
            signatures: vec![],
            rooms: vec![],
            aids: "".to_string(),
            moment: "Laboration 1 grupp A och B".to_string(),
            activity_type: ActivityType::Lab,
            groups: vec!["grupp a".to_string(), "grupp b".to_string(), "grupp a och b".to_string()],
            urls: vec![],
            updated: "".to_string(),
        };

        let lab_cd = Event {
            week: 36,
            week_year: 2026,
            day: "Tis".to_string(),
            date: "1 Sep".to_string(),
            full_date: "2026-09-01".to_string(),
            time_span: "13:15-17:00".to_string(),
            start_time: "13:15".to_string(),
            end_time: "17:00".to_string(),
            duration_display: "3h 45m".to_string(),
            course: "Kemi".to_string(),
            course_code: "40S01A".to_string(),
            signatures: vec![],
            rooms: vec![],
            aids: "".to_string(),
            moment: "Laboration 1 grupp C och D".to_string(),
            activity_type: ActivityType::Lab,
            groups: vec!["grupp c".to_string(), "grupp d".to_string(), "grupp c och d".to_string()],
            urls: vec![],
            updated: "".to_string(),
        };

        // Case 1: Empty filters (Alla) -> matches all events
        let empty_filters: Vec<String> = vec![];
        assert!(lecture.matches_groups(&empty_filters));
        assert!(exercise_g1.matches_groups(&empty_filters));
        assert!(exercise_g2.matches_groups(&empty_filters));
        assert!(lab_ab.matches_groups(&empty_filters));
        assert!(lab_cd.matches_groups(&empty_filters));

        // Case 2: Selected "grupp 1" and "grupp a"
        let my_groups = vec!["grupp 1".to_string(), "grupp a".to_string()];
        // Lecture has no group tag -> always shown
        assert!(lecture.matches_groups(&my_groups));
        // Exercise G1 -> matches!
        assert!(exercise_g1.matches_groups(&my_groups));
        // Exercise G2 -> filtered out!
        assert!(!exercise_g2.matches_groups(&my_groups));
        // Lab AB -> matches "grupp a"!
        assert!(lab_ab.matches_groups(&my_groups));
        // Lab CD -> filtered out!
        assert!(!lab_cd.matches_groups(&my_groups));
    }
}
