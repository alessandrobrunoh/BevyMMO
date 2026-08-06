//! Layered configuration: defaults <- `config/default.toml` <-
//! `config/<APP_ENV>.toml` <- `config/local.toml` <- env vars.
//!
//! I secret non vanno committati: usare `config/local.toml` (gitignored)
//! o env vars (es. `DATABASE_URL`).

use serde::Deserialize;

const DEFAULT_TOML: &str = include_str!("../config/default.toml");

/// Configurazione dell'applicazione caricata dai file `config/*.toml`
/// e dalle env vars.
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    /// Stringa di filtro per `LogPlugin` nel formato `RUST_LOG`.
    #[serde(default = "default_log_filter")]
    pub log_filter: String,

    /// Frequenza del tick fisso (Hz). Il `Fixed` schedule usa `1.0 / tick_rate`.
    #[serde(default = "default_tick_rate")]
    pub tick_rate: f64,

    /// URL di connessione a PostgreSQL, nella forma
    /// `postgresql://user:password@host:port/db`.
    ///
    /// Obbligatorio in modalita' server: fornito da `config/<env>.toml`,
    /// `config/local.toml`, o dalla env var `DATABASE_URL`.
    #[serde(default)]
    pub database_url: Option<String>,

    #[serde(default)]
    pub server: ServerSettings,

    #[serde(default)]
    pub client: ClientSettings,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ServerSettings {
    /// Indirizzo locale su cui il server ascolta (bind), come stringa
    /// "host:port". Mantenuto come `String` perche' la crate `config`
    /// deserializza da TOML/env come stringa; la conversione in `SocketAddr`
    /// avviene nei chiamanti.
    #[serde(default = "default_server_bind_addr")]
    pub bind_addr: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClientSettings {
    /// Indirizzo del server remoto a cui connettersi ("host:port").
    #[serde(default = "default_client_server_addr")]
    pub server_addr: String,

    /// Indirizzo locale bindato dal client (tipicamente "0.0.0.0:0").
    #[serde(default = "default_client_addr")]
    pub client_addr: String,
}

impl Settings {
    /// Carica la configurazione unendo le sorgenti nell'ordine:
    /// `default.toml` < `config/<APP_ENV>.toml` < `config/local.toml` < env vars.
    ///
    /// `APP_ENV` e' letto da env var (default: `development`).
    pub fn load() -> Self {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_owned());

        let builder = config::Config::builder()
            // Defaults committati.
            .add_source(config::File::from_str(
                DEFAULT_TOML,
                config::FileFormat::Toml,
            ))
            // Profilo corrente (development/production/...). Non required:
            // se manca si usano solo i defaults.
            .add_source(config::File::with_name(&format!("config/{env}")).required(false))
            // Override locali gitignored.
            .add_source(config::File::with_name("config/local").required(false))
            // Env vars sovrascrivono tutto. `try_parsing(true)` lascia che
            // valori come "1.0" vengano interpretati come numero dove serve.
            .add_source(config::Environment::default().try_parsing(true));

        builder
            .build()
            .expect("failed to build configuration")
            .try_deserialize()
            .expect("failed to deserialize configuration")
    }
}

fn default_log_filter() -> String {
    "warn,bevy_lightyear_game=debug,lightyear=info".to_owned()
}

fn default_tick_rate() -> f64 {
    60.0
}

fn default_server_bind_addr() -> String {
    "0.0.0.0:5051".to_owned()
}

fn default_client_server_addr() -> String {
    "127.0.0.1:5051".to_owned()
}

fn default_client_addr() -> String {
    "0.0.0.0:0".to_owned()
}
