use crate::config::AppConfig;
use crate::parser::Schedule;
use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub struct FetchResult {
    pub schedule: Schedule,
    pub from_cache: bool,
    pub cache_age: Option<Duration>,
    pub warning: Option<String>,
}

pub struct ScheduleFetcher {
    client: reqwest::blocking::Client,
    cache_ttl: Duration,
    cache_dir: PathBuf,
}

impl ScheduleFetcher {
    pub fn new(cache_ttl_minutes: u64) -> Result<Self> {
        let cache_dir = AppConfig::get_cache_dir()?;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (compatible; HB-Schema-TUI/0.1.0; +https://github.com/lejon/hb-schema)")
            .build()?;

        Ok(Self {
            client,
            cache_ttl: Duration::from_secs(cache_ttl_minutes * 60),
            cache_dir,
        })
    }

    fn cache_path_for_url(&self, url: &str) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();
        self.cache_dir.join(format!("schedule_{:016x}.html", hash))
    }

    pub fn fetch(&self, url: &str, force_refresh: bool) -> Result<FetchResult> {
        let cache_path = self.cache_path_for_url(url);

        // Check cache
        if !force_refresh && cache_path.exists() {
            if let Ok(metadata) = fs::metadata(&cache_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = SystemTime::now().duration_since(modified) {
                        if age < self.cache_ttl {
                            if let Ok(content) = fs::read_to_string(&cache_path) {
                                if let Ok(schedule) = Schedule::parse_html(&content, Some(url)) {
                                    return Ok(FetchResult {
                                        schedule,
                                        from_cache: true,
                                        cache_age: Some(age),
                                        warning: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Try network request
        match self.client.get(url).send() {
            Ok(response) => {
                if response.status().is_success() {
                    let html = response.text().context("Kunde inte läsa svarstext från servern")?;
                    // Save to cache
                    let _ = fs::write(&cache_path, &html);
                    let schedule = Schedule::parse_html(&html, Some(url))?;
                    Ok(FetchResult {
                        schedule,
                        from_cache: false,
                        cache_age: None,
                        warning: None,
                    })
                } else {
                    let status = response.status();
                    // Fallback to cache if available
                    if cache_path.exists() {
                        let content = fs::read_to_string(&cache_path)?;
                        let schedule = Schedule::parse_html(&content, Some(url))?;
                        Ok(FetchResult {
                            schedule,
                            from_cache: true,
                            cache_age: None,
                            warning: Some(format!("Servern svarade med felkod {}. Visar cachad version.", status)),
                        })
                    } else {
                        anyhow::bail!("Servern svarade med felkod: {}", status);
                    }
                }
            }
            Err(err) => {
                // Offline fallback
                if cache_path.exists() {
                    let content = fs::read_to_string(&cache_path)?;
                    let schedule = Schedule::parse_html(&content, Some(url))?;
                    Ok(FetchResult {
                        schedule,
                        from_cache: true,
                        cache_age: None,
                        warning: Some(format!("Kunde inte ansluta till nätverket ({}). Visar offline-cache.", err)),
                    })
                } else {
                    Err(anyhow::anyhow!("Kunde inte hämta schemat och ingen offline-cache hittades: {}", err))
                }
            }
        }
    }

    pub fn download_ical(&self, ical_url: &str, output_path: &PathBuf) -> Result<()> {
        let resp = self.client.get(ical_url).send()?.error_for_status()?;
        let bytes = resp.bytes()?;
        fs::write(output_path, bytes)?;
        Ok(())
    }
}
