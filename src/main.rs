use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::window::PresentMode;
use clap::{Parser, Subcommand};
use core::time::Duration;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

mod game_state;
mod network;
mod persistence;
mod plugins;
mod scenes;
mod ui;

const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5050);
const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
const FIXED_TIMESTEP: f64 = 60.0;
const LOG_FILTER: &str = "warn,bevy_lightyear_game=debug,lightyear=info";

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    Client {
        /// Override riproducibile dell'identità Netcode. Se omesso, viene
        /// generato un ID non-zero unico per questo processo.
        #[arg(short, long)]
        client_id: Option<u64>,
    },
    Server,
    HostClient {
        /// Override riproducibile dell'identità Netcode. Se omesso, viene
        /// generato un ID non-zero unico per questo processo.
        #[arg(short, long)]
        client_id: Option<u64>,
    },
}

struct AppConfig {
    mode: network::mode::AppMode,
    client_id: Option<u64>,
}

impl AppConfig {
    fn from_cli(mode: Mode) -> Self {
        match mode {
            Mode::Client { client_id } => Self {
                mode: network::mode::AppMode::Client,
                client_id: Some(resolve_client_id(client_id)),
            },
            Mode::Server => Self {
                mode: network::mode::AppMode::Server,
                client_id: None,
            },
            Mode::HostClient { client_id } => Self {
                mode: network::mode::AppMode::HostClient,
                client_id: Some(resolve_client_id(client_id)),
            },
        }
    }

    fn client_id(&self) -> u64 {
        self.client_id
            .expect("client and host-client modes always resolve a client id")
    }
}

/// Produce un ID Netcode distinto per processi locali concorrenti.
///
/// Il server rifiuta due client con lo stesso ID; `0` viene evitato perché è
/// l'identità placeholder usata da Lightyear per l'autenticazione assente.
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
    let config = AppConfig::from_cli(Cli::parse().mode);
    println!("Starting {:?}", config.mode);
    build_app(&config).run();
}

fn build_app(config: &AppConfig) -> App {
    let mut app = App::new();
    add_platform_plugins(&mut app, config);
    app.insert_resource(config.mode);
    app.add_plugins(game_state::GameStatePlugin);

    let tick_duration = Duration::from_secs_f64(1.0 / FIXED_TIMESTEP);

    if config.mode.has_server() {
        app.add_plugins(persistence::PersistencePlugin);
        app.add_plugins(network::server::ServerPlugins {
            server_addr: SERVER_ADDR,
            tick_duration,
        });
    }

    if config.mode.has_client() {
        app.add_plugins(network::client::ClientPlugins {
            client_id: config.client_id(),
            server_addr: SERVER_ADDR,
            client_addr: CLIENT_ADDR,
            tick_duration,
        });
    }

    app.add_plugins(network::protocol::ProtocolPlugin);
    app.add_plugins(plugins::entity::EntityPlugin);
    app.add_plugins(plugins::player_movement::PlayerMovementPlugin);

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
        app.add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    filter: LOG_FILTER.to_string(),
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
    } else {
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(LogPlugin {
            filter: LOG_FILTER.to_string(),
            ..default()
        });
        app.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f64(
            1.0 / FIXED_TIMESTEP,
        )));
    }
}
