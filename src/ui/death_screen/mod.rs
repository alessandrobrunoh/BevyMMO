//! Death screen overlay: mostra "Sei morto" e un pulsante `Respawn` quando il
//! player locale è in `EntityState::Dead`.

mod plugin;
mod systems;

pub use plugin::DeathScreenPlugin;
