use bevy::log::LogPlugin;
use bevy::prelude::*;
#[cfg(feature = "client")]
use bevy::window::PresentMode;
use clap::{Parser, Subcommand};
use core::time::Duration;

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
    /// Run the game client.
    ///
    /// There is no `server` or `host-client` any more: the authoritative server
    /// is the SpacetimeDB module (`docker compose up -d spacetimedb`, then
    /// `./scripts/stdb.sh publish`), not a Bevy process.
    Client {
        /// SpacetimeDB URL override (default: from config).
        #[arg(long)]
        uri: Option<String>,

        /// Published module name override (default: from config).
        #[arg(long)]
        module: Option<String>,
    },
}

struct AppConfig {
    mode: mode::AppMode,
    log_filter: String,
    tick_rate: f64,
    spacetime_uri: String,
    spacetime_module: String,
}

impl AppConfig {
    /// Merges default values from `config/*.toml` files with CLI overrides.
    fn resolve(mode: Mode, settings: Settings) -> Self {
        let Mode::Client { uri, module } = mode;
        Self {
            mode: mode::AppMode::Client,
            tick_rate: settings.tick_rate,
            log_filter: settings.log_filter.clone(),
            spacetime_uri: uri.unwrap_or(settings.spacetime_uri),
            spacetime_module: module.unwrap_or(settings.spacetime_module),
        }
    }
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

    #[cfg(feature = "client")]
    if config.mode.has_client() {
        app.add_plugins(bevymmo_client::stdb::StdbPlugin {
            uri: config.spacetime_uri.clone(),
            module: config.spacetime_module.clone(),
        });
    }

    // Local Bevy messages/resources used by client presentation systems. The
    // Lightyear protocol registration alone does not initialize the local
    // message queue or the shared spell registry.
    app.add_message::<bevymmo_shared::network::protocol::SpellVisualEffect>();
    app.add_message::<bevymmo_shared::network::protocol::SpellCastProgress>();
    app.add_message::<bevymmo_shared::network::protocol::SpellCastEnded>();
    // Written by the SpacetimeDB bridge, read by the presentation: the server's
    // refusals and announcements, and the authoritative cooldown table.
    app.add_message::<bevymmo_shared::server_feed::ServerNotice>();
    app.add_message::<bevymmo_shared::server_feed::SpellCooldownState>();
    // Game content. These used to be empty `Resource`s filled by `Startup`
    // systems; they are plain values now, because the SpacetimeDB module needs
    // the same registries and has no ECS to build them in. Inserting them
    // directly also removes a startup-ordering hazard: nothing can read a
    // registry before the system that populates it has run, because there is no
    // such system.
    app.insert_resource(bevymmo_shared::spells_impl::default_spells());
    app.insert_resource(bevymmo_shared::items_impl::default_items());
    app.insert_resource(bevymmo_shared::base_abilities_impl::default_base_abilities());
    app.insert_resource(bevymmo_shared::essences_impl::default_essences());
    app.insert_resource(bevymmo_shared::modifiers_impl::default_modifiers());
    app.insert_resource(bevymmo_shared::ancient_words_impl::default_ancient_words());

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
            // Names the window after the module it is talking to. The old title
            // carried a Netcode client id, which no longer exists: SpacetimeDB
            // identifies a connection by its `Identity`, and the client does not
            // know that until it has connected.
            let title = format!("BevyMMO — {}", config.spacetime_module);
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


