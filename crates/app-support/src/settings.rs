//! Layered configuration: defaults <- `config/default.toml` <-
//! `config/<APP_ENV>.toml` <- `config/local.toml` <- env vars.
//!
//! Secrets should not be committed: use `config/local.toml` (gitignored)
//! or env vars (e.g. `DATABASE_URL`).

use serde::Deserialize;

const DEFAULT_TOML: &str = include_str!("../../../config/default.toml");

/// Application configuration loaded from `config/*.toml` files
/// and env vars.
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    /// Filter string for `LogPlugin` matching `RUST_LOG` format.
    #[serde(default = "default_log_filter")]
    pub log_filter: String,

    /// Fixed tick rate (Hz). The `Fixed` schedule uses `1.0 / tick_rate`.
    #[serde(default = "default_tick_rate")]
    pub tick_rate: f64,

    /// PostgreSQL connection URL, in the format
    /// `postgresql://user:password@host:port/db`.
    ///
    /// Required in server mode: provided by `config/<env>.toml`,
    /// `config/local.toml`, or env var `DATABASE_URL`.
    #[serde(default)]
    pub database_url: Option<String>,

    /// WebSocket URL of the SpacetimeDB instance.
    ///
    /// Set alongside `database_url` rather than replacing it: the migration to
    /// SpacetimeDB is incremental, and the lightyear/Postgres stack has to keep
    /// working until the gameplay reducers are ported. `database_url` goes away
    /// with `crates/server`.
    #[serde(default = "default_spacetime_uri")]
    pub spacetime_uri: String,

    /// Name the module is published under (`spacetime publish <name>`).
    #[serde(default = "default_spacetime_module")]
    pub spacetime_module: String,

    #[serde(default)]
    pub server: ServerSettings,

    #[serde(default)]
    pub client: ClientSettings,

    #[serde(default)]
    pub gateway: GatewaySettings,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ServerSettings {
    /// Local address the server listens on (bind), as string
    /// "host:port". Maintained as `String` because `config` crate
    /// deserializes from TOML/env as string; conversion to `SocketAddr`
    /// happens at call sites.
    #[serde(default = "default_server_bind_addr")]
    pub bind_addr: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClientSettings {
    /// Address of the remote server to connect to ("host:port").
    #[serde(default = "default_client_server_addr")]
    pub server_addr: String,

    /// Local address bound by the client (typically "0.0.0.0:0").
    #[serde(default = "default_client_addr")]
    pub client_addr: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GatewaySettings {
    /// Local address the HTTP gateway binds to ("host:port").
    ///
    /// Kept as `String` for the same reason as `ServerSettings::bind_addr`:
    /// the `config` crate hands us strings, conversion happens at call sites.
    #[serde(default = "default_gateway_bind_addr")]
    pub bind_addr: String,
}

impl Settings {
    /// Loads configuration merging sources in order:
    /// `default.toml` < `config/<APP_ENV>.toml` < `config/local.toml` < env vars.
    ///
    /// `APP_ENV` is read from env var (default: `development`).
    pub fn load() -> Self {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_owned());

        let builder = config::Config::builder()
            // Committed defaults.
            .add_source(config::File::from_str(
                DEFAULT_TOML,
                config::FileFormat::Toml,
            ))
            // Current profile (development/production/...). Not required:
            // if missing, defaults are used.
            .add_source(config::File::with_name(&format!("config/{env}")).required(false))
            // Local gitignored overrides.
            .add_source(config::File::with_name("config/local").required(false))
            // Env vars override everything. `try_parsing(true)` lets
            // values like "1.0" be parsed as numbers where needed.
            // Double underscores (e.g. `GATEWAY__BIND_ADDR`) map to nested fields.
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .try_parsing(true),
            );

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

fn default_spacetime_uri() -> String {
    "ws://127.0.0.1:3000".to_string()
}

fn default_spacetime_module() -> String {
    "bevymmo".to_string()
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

fn default_gateway_bind_addr() -> String {
    "127.0.0.1:8080".to_owned()
}
