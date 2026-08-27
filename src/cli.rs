use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hb-schema")]
#[command(author = "HB Schema TUI Team")]
#[command(version = "0.1.0")]
#[command(about = "Snyggt och smidigt TUI-program för Högskolan i Borås (HB) scheman (KronoX)", long_about = None)]
pub struct CliArgs {
    /// Sökväg till konfigurationsfil (standard: ~/.config/hb-schema/config.toml eller ./config.toml)
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Välj en specifik profil från konfigurationsfilen
    #[arg(short, long)]
    pub profile: Option<String>,

    /// Ladda en direkt KronoX schema-URL istället för konfiguration
    #[arg(short, long)]
    pub url: Option<String>,

    /// Förvalt gruppfilter (t.ex. "grupp 1", "grupp 2", "alla")
    #[arg(short, long)]
    pub group: Option<String>,

    /// Tvinga omladdning från servern (ignorera lokal cache)
    #[arg(short, long)]
    pub refresh: bool,

    /// Skriv ut dagens schema i terminalen och avsluta (icke-interaktivt)
    #[arg(long)]
    pub today: bool,

    /// Skriv ut angiven vecka i terminalen och avsluta (t.ex. --week 36)
    #[arg(long)]
    pub week: Option<Option<u32>>,

    /// Exportera/ladda ner iCal (.ics) kalenderfil till angiven sökväg
    #[arg(long)]
    pub export_ical: Option<PathBuf>,
}
