use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const QUALIFIER: &str = "se";
const ORGANIZATION: &str = "hb";
const APPLICATION: &str = "schema";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_profile_name")]
    pub default_profile: String,

    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_minutes: u64,

    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

fn default_profile_name() -> String {
    "tekniskt_basar".to_string()
}

fn default_cache_ttl() -> u64 {
    180 // 3 hours
}

fn default_theme() -> String {
    "modern".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub url: String,
    #[serde(default = "default_all_filter")]
    pub group_filter: String,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_all_filter() -> String {
    "alla".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "tekniskt_basar".to_string(),
            Profile {
                name: "Tekniskt Basår (KBAST26h)".to_string(),
                url: "https://schema.hb.se/setup/jsp/Schema.jsp?startDatum=2026-08-31&intervallTyp=a&intervallAntal=1&sprak=SV&sokMedAND=true&forklaringar=true&resurser=p.KBAST26h".to_string(),
                group_filter: "alla".to_string(),
                description: Some("Högskolan i Borås - Tekniskt basår HT26/VT27".to_string()),
            },
        );
        profiles.insert(
            "dataing".to_string(),
            Profile {
                name: "Högskoleingenjör Datateknik (TGITI26h)".to_string(),
                url: "https://schema.hb.se/setup/jsp/Schema.jsp?startDatum=2026-08-31&intervallTyp=a&intervallAntal=1&sprak=SV&sokMedAND=true&forklaringar=true&resurser=p.TGITI26h".to_string(),
                group_filter: "alla".to_string(),
                description: Some("Högskolan i Borås - Datateknik HT26/VT27".to_string()),
            },
        );

        Self {
            default_profile: "tekniskt_basar".to_string(),
            cache_ttl_minutes: 180,
            theme: "modern".to_string(),
            profiles,
        }
    }
}

impl AppConfig {
    pub fn load_or_create(custom_path: Option<&Path>) -> Result<(Self, PathBuf)> {
        if let Some(path) = custom_path {
            if path.exists() {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Kunde inte läsa konfigurationsfil: {:?}", path))?;
                let config: AppConfig = toml::from_str(&content)
                    .with_context(|| format!("Ogiltigt TOML-format i {:?}", path))?;
                return Ok((config, path.to_path_buf()));
            } else {
                let config = AppConfig::default();
                config.save(path)?;
                return Ok((config, path.to_path_buf()));
            }
        }

        let local_config = PathBuf::from("config.toml");
        if local_config.exists() {
            let content = fs::read_to_string(&local_config)
                .with_context(|| "Kunde inte läsa lokal config.toml")?;
            let config: AppConfig = toml::from_str(&content)
                .with_context(|| "Ogiltigt format i lokal config.toml")?;
            return Ok((config, local_config));
        }

        let config_path = Self::get_default_config_path()?;
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Kunde inte läsa konfiguration: {:?}", config_path))?;
            let config: AppConfig = toml::from_str(&content)
                .with_context(|| format!("Ogiltigt format i {:?}", config_path))?;
            return Ok((config, config_path));
        }

        let config = AppConfig::default();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        config.save(&config_path)?;
        Ok((config, config_path))
    }

    pub fn get_default_config_path() -> Result<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) {
            let config_dir = proj_dirs.config_dir();
            Ok(config_dir.join("config.toml"))
        } else {
            Ok(PathBuf::from("config.toml"))
        }
    }

    pub fn get_cache_dir() -> Result<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) {
            let cache_dir = proj_dirs.cache_dir();
            fs::create_dir_all(cache_dir)?;
            Ok(cache_dir.to_path_buf())
        } else {
            let local_cache = PathBuf::from(".cache");
            fs::create_dir_all(&local_cache)?;
            Ok(local_cache)
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let toml_str = toml::to_string_pretty(self)
            .context("Kunde inte serialisera konfiguration till TOML")?;
        
        let header = r#"# ==========================================================
# Högskolan i Borås (HB) - Schema TUI Konfigurationsfil
# ==========================================================
#
# Du kan lägga till flera scheman under [profiles.<namn>]
# och snabbt växla mellan dem med tangenten 's' i programmet.
#
"#;
        let full_content = format!("{}{}", header, toml_str);
        fs::write(path, full_content)
            .with_context(|| format!("Kunde inte spara konfigurationsfil till {:?}", path))?;
        Ok(())
    }

    pub fn get_active_profile<'a>(&'a self, override_name: Option<&'a str>) -> Option<(&'a str, &'a Profile)> {
        if let Some(name) = override_name {
            if let Some(p) = self.profiles.get(name) {
                return Some((name, p));
            }
        }
        if let Some(p) = self.profiles.get(&self.default_profile) {
            return Some((&self.default_profile, p));
        }
        self.profiles.iter().next().map(|(k, v)| (k.as_str(), v))
    }
}
