use bevy::log::LogPlugin;
use bevy::prelude::*;
#[cfg(feature = "client")]
use bevy::window::PresentMode;
use clap::{Parser, Subcommand};
use core::time::Duration;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use bevymmo_shared::paths;
use bevymmo_shared::settings::Settings;
use bevymmo_shared::{game_state, network::mode};

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
    Editor,
}

struct AppConfig {
    mode: mode::AppMode,
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
                mode: mode::AppMode::Client,
                client_id: Some(resolve_client_id(client_id)),
                server_addr: server_addr.unwrap_or(client_server_addr),
                client_addr,
                tick_rate,
                log_filter,
                database_url,
            },
            Mode::Server { bind_addr } => Self {
                mode: mode::AppMode::Server,
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
                mode: mode::AppMode::HostClient,
                client_id: Some(resolve_client_id(client_id)),
                server_addr: server_addr.unwrap_or(client_server_addr),
                client_addr,
                tick_rate,
                log_filter,
                database_url,
            },
            Mode::Editor => Self {
                mode: mode::AppMode::Editor,
                client_id: None,
                server_addr: client_server_addr,
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

    if matches!(config.mode, mode::AppMode::Editor) {
        #[cfg(feature = "editor")]
        {
            app.add_plugins(bevymmo_editor::EditorPlugin);
            return app;
        }

        #[cfg(not(feature = "editor"))]
        {
            panic!("editor mode requires the 'editor' cargo feature to be enabled");
        }
    }

    let tick_duration = Duration::from_secs_f64(1.0 / config.tick_rate);

    if config.mode.has_server() {
        let database_url = config
            .database_url
            .clone()
            .expect("DATABASE_URL is required when starting a server; set it in config/<env>.toml, config/local.toml, or as the DATABASE_URL env var");
        app.add_plugins(bevymmo_server::ServerPlugin {
            database_url,
            server_addr: config.server_addr,
            tick_duration,
        });
    }

    #[cfg(feature = "client")]
    if config.mode.has_client() {
        app.add_plugins(bevymmo_client::network::client::ClientTransportPlugins {
            client_id: config.client_id(),
            server_addr: config.server_addr,
            client_addr: config.client_addr,
            tick_duration,
        });
        app.add_systems(
            Startup,
            (
                bevymmo_shared::spells_impl::register_default_spells,
                bevymmo_shared::items_impl::register_default_items,
            ),
        );
    }

    app.add_plugins(bevymmo_shared::network::protocol::ProtocolPlugin);
    // Local Bevy messages/resources used by client presentation systems. The
    // Lightyear protocol registration alone does not initialize the local
    // message queue or the shared spell registry.
    app.add_message::<bevymmo_shared::network::protocol::SpellVisualEffect>();
    app.add_message::<bevymmo_shared::network::protocol::SpellCastProgress>();
    app.add_message::<bevymmo_shared::network::protocol::SpellCastEnded>();
    app.init_resource::<bevymmo_shared::spells::SpellRegistry>();
    app.init_resource::<bevymmo_shared::items::registry::ItemRegistry>();

    #[cfg(feature = "client")]
    if config.mode.has_client() {
        app.add_plugins(bevymmo_client::player_movement::PlayerMovementPlugin);
        app.add_plugins(bevymmo_client::targeting::TargetingPlugin);
        // Load client-side model/animation asset collections before renderer
        // systems try to spawn player and enemy scenes.
        app.add_plugins(bevymmo_presentation::PresentationCorePlugin);
        app.add_plugins(bevymmo_presentation::PresentationPlugin);
    }

    app
}

fn add_platform_plugins(app: &mut App, config: &AppConfig) {
    if config.mode.is_windowed() {
        #[cfg(feature = "client")]
        {
            let title = match config.client_id {
                Some(client_id) => format!("{:?} {}", config.mode, client_id),
                None => format!("{:?}", config.mode),
            };
            app.add_plugins(
                DefaultPlugins
                    // The runnable package lives in `bins/game`, while the
                    // workspace keeps shared assets at the repository root.
                    // The assets folder is resolved by walking up from the
                    // executable / working directory so the same binary works
                    // whether launched via `cargo run` or by double-clicking
                    // `target/debug/game.exe`.
                    .set(AssetPlugin {
                        file_path: paths::assets_root().to_string_lossy().into_owned(),
                        ..default()
                    })
                    .set(LogPlugin {
                        filter: config.log_filter.clone(),
                        ..default()
                    })
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title,
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
            panic!("windowed modes require the 'client' cargo feature to be enabled");
        }
    } else {
        let tick_duration = Duration::from_secs_f64(1.0 / config.tick_rate);
        app.add_plugins((
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(tick_duration)),
            bevy::state::app::StatesPlugin,
        ));
        app.add_plugins(LogPlugin {
            filter: config.log_filter.clone(),
            ..default()
        });
        app.insert_resource(Time::<Fixed>::from_duration(tick_duration));
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
