use bevy::log::LogPlugin;
use bevy::prelude::*;
#[cfg(feature = "client")]
use bevy::window::PresentMode;
use clap::{Parser, Subcommand};
use core::time::Duration;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

mod game_state;
mod migrations;
mod network;
mod plugins;
#[cfg(feature = "client")]
mod scenes;
mod settings;
mod spells;
mod stats;
#[cfg(feature = "client")]
mod ui;

use settings::Settings;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    Client {
        /// Reproducible Netcode identity override. If omitted, a
        /// unique non-zero ID is generated for this process.
        #[arg(short, long)]
        client_id: Option<u64>,

        /// Remote server address override to connect to
        /// (default: from config/client.toml).
        #[arg(long)]
        server_addr: Option<SocketAddr>,
    },
    Server {
        /// Local address override for the server to listen on
        /// (default: from config/server.toml).
        #[arg(long)]
        bind_addr: Option<SocketAddr>,
    },
    HostClient {
        /// Reproducible Netcode identity override. If omitted, a
        /// unique non-zero ID is generated for this process.
        #[arg(short, long)]
        client_id: Option<u64>,

        /// Local server address override used by the embedded client
        /// (default: from config/client.toml).
        #[arg(long)]
        server_addr: Option<SocketAddr>,
    },
}

struct AppConfig {
    mode: network::mode::AppMode,
    client_id: Option<u64>,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    log_filter: String,
    tick_rate: f64,
    database_url: Option<String>,
}

impl AppConfig {
    /// Merges default values from `config/*.toml` files with CLI overrides.
    fn resolve(mode: Mode, settings: Settings) -> Self {
        let tick_rate = settings.tick_rate;
        let log_filter = settings.log_filter.clone();
        let database_url = settings.database_url.clone();

        // `SocketAddr` values arrive as strings from `Settings`; we parse them
        // once here. Defaults guarantee valid values.
        let client_server_addr = parse_addr(&settings.client.server_addr, "client.server_addr");
        let client_addr = parse_addr(&settings.client.client_addr, "client.client_addr");
        let server_bind_addr = parse_addr(&settings.server.bind_addr, "server.bind_addr");

        match mode {
            Mode::Client {
                client_id,
                server_addr,
            } => Self {
                mode: network::mode::AppMode::Client,
                client_id: Some(resolve_client_id(client_id)),
                server_addr: server_addr.unwrap_or(client_server_addr),
                client_addr,
                tick_rate,
                log_filter,
                database_url,
            },
            Mode::Server { bind_addr } => Self {
                mode: network::mode::AppMode::Server,
                client_id: None,
                server_addr: bind_addr.unwrap_or(server_bind_addr),
                client_addr,
                tick_rate,
                log_filter,
                database_url,
            },
            Mode::HostClient {
                client_id,
                server_addr,
            } => Self {
                mode: network::mode::AppMode::HostClient,
                client_id: Some(resolve_client_id(client_id)),
                server_addr: server_addr.unwrap_or(client_server_addr),
                client_addr,
                tick_rate,
                log_filter,
                database_url,
            },
        }
    }

    fn client_id(&self) -> u64 {
        self.client_id
            .expect("client and host-client modes always resolve a client id")
    }
}

/// Produces a distinct Netcode ID for concurrent local processes.
///
/// The server rejects two clients with the same ID; `0` is avoided because it is
/// the placeholder identity used by Lightyear for missing authentication.
fn resolve_client_id(override_id: Option<u64>) -> u64 {
    if let Some(id) = override_id.filter(|id| *id != 0) {
        return id;
    }

    static SEQUENCE: AtomicU64 = AtomicU64::new(1);

    let process_id = u64::from(std::process::id());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or_default();
    let generated = (process_id << 32) ^ timestamp ^ SEQUENCE.fetch_add(1, Ordering::Relaxed);

    generated.max(1)
}

fn main() {
    let settings = Settings::load();
    let config = AppConfig::resolve(Cli::parse().mode, settings);
    println!("Starting {:?}", config.mode);
    build_app(&config).run();
}

fn build_app(config: &AppConfig) -> App {
    let mut app = App::new();
    add_platform_plugins(&mut app, config);
    app.insert_resource(config.mode);
    app.add_plugins(game_state::GameStatePlugin);

    let tick_duration = Duration::from_secs_f64(1.0 / config.tick_rate);

    if config.mode.has_server() {
        let database_url = config
            .database_url
            .clone()
            .expect("DATABASE_URL is required when starting a server; set it in config/<env>.toml, config/local.toml, or as the DATABASE_URL env var");
        app.add_plugins(plugins::persistence::PersistencePlugin::new(database_url));
        app.add_plugins(network::server::ServerPlugins {
            server_addr: config.server_addr,
            tick_duration,
        });
    }

    #[cfg(feature = "client")]
    if config.mode.has_client() {
        app.add_plugins(network::client::ClientPlugins {
            client_id: config.client_id(),
            server_addr: config.server_addr,
            client_addr: config.client_addr,
            tick_duration,
        });
    }

    app.add_plugins(network::protocol::ProtocolPlugin);
    app.add_plugins(stats::StatsPlugin);
    app.add_plugins(plugins::entity::EntityPlugin);
    app.add_plugins(plugins::player_movement::PlayerMovementPlugin);
    app.add_plugins(plugins::crowd_control::CrowdControlPlugin);
    #[cfg(feature = "client")]
    app.add_plugins(plugins::targeting::TargetingPlugin);
    app.add_plugins(plugins::spells::SpellsPlugin);

    #[cfg(feature = "client")]
    if config.mode.has_client() {
        app.add_plugins(plugins::key_mapping::KeyMappingPlugin);
        app.add_plugins(ui::UiPlugin);
        app.add_plugins((scenes::ScenesPlugin, plugins::renderer::RendererPlugin));
    }

    app
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_client_ids_are_non_zero_and_distinct() {
        let first = resolve_client_id(None);
        let second = resolve_client_id(None);

        assert_ne!(first, 0);
        assert_ne!(second, 0);
        assert_ne!(first, second);
    }

    #[test]
    fn explicit_non_zero_client_id_is_preserved() {
        assert_eq!(resolve_client_id(Some(42)), 42);
    }

    #[test]
    fn zero_client_id_is_replaced_with_a_generated_value() {
        assert_ne!(resolve_client_id(Some(0)), 0);
    }
}

fn add_platform_plugins(app: &mut App, config: &AppConfig) {
    if config.mode.has_client() {
        #[cfg(feature = "client")]
        {
            app.add_plugins(
                DefaultPlugins
                    .set(LogPlugin {
                        filter: config.log_filter.clone(),
                        ..default()
                    })
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: format!("{:?} {}", config.mode, config.client_id()),
                            resolution: (800, 600).into(),
                            present_mode: PresentMode::AutoVsync,
                            ..default()
                        }),
                        ..default()
                    }),
            );
        }
        #[cfg(not(feature = "client"))]
        {
            let _ = config;
            panic!("client mode requires the 'client' cargo feature to be enabled");
        }
    } else {
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(LogPlugin {
            filter: config.log_filter.clone(),
            ..default()
        });
        app.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f64(
            1.0 / config.tick_rate,
        )));
    }
}

/// Parses a "host:port" address from configuration files.
///
/// Failing here is a configuration error: it is better to panic on startup with
/// a clear message than at runtime.
fn parse_addr(raw: &str, field: &str) -> SocketAddr {
    raw.parse()
        .unwrap_or_else(|e| panic!("invalid socket address for {field}: {raw:?} ({e})"))
}

