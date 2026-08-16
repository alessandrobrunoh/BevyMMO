//! Client-only modular ability HUD for Eidolon weapons.
//!
//! Data-driven UI showing weapon ability slots (Q/W/E), keybinds and cooldowns.
//! Entries are derived exclusively from `WeaponAbilities`; the legacy
//! `SpellHotbar` / `SpellRegistry` path is no longer used for player input.

use bevy::prelude::*;
use std::collections::HashMap;

use bevymmo_gameplay::abilities::{
    resolve_active_ability, AbilityId, AbilitySlot, BaseAbilityRegistry, EssenceRegistry,
};
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_gameplay::items::components::Equipment;
use bevymmo_gameplay::items::registry::ItemRegistry;
use bevymmo_network::network::mode::has_client;
use bevymmo_network::network::protocol::NetworkEntityId;
use bevymmo_client::server_feed::SpellCooldownState;
use bevymmo_client::user_settings::{GameSettingsResource, KeyAction};

use crate::game_state::{GameScreen, Screen};
use crate::ui::theme::UiTheme;

/// What a HUD cooldown countdown is keyed by.
///
/// Only `AbilityId` exists now — the legacy `SpellId` variant was removed when
/// player input consolidated onto the Eidolon pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HudCooldownKey {
    Ability(AbilityId),
}

#[derive(Message, Debug, Clone, PartialEq)]
pub struct SpellHudCooldownStarted {
    pub key: HudCooldownKey,
    pub cooldown_seconds: f32,
}

#[derive(Resource, Default)]
pub struct SpellHudState {
    remaining_seconds: HashMap<HudCooldownKey, f32>,
}

#[derive(Resource, Default)]
struct SpellHudLayoutState {
    initialized: bool,
    /// `(ability_slot, ability_id, key_label, display_name)` — rebuilds the HUD
    /// when any of these change (e.g. weapon swap or Incisione rewrite).
    signature: Vec<(AbilitySlot, Option<AbilityId>, String, String)>,
}

impl SpellHudState {
    /// Returns true if whatever `key` names is still on cooldown on the client.
    ///
    /// This is used to gate local cast feedback (visuals, HUD) so the player
    /// cannot spam the cast key while waiting for the server-validated
    /// cooldown to expire.
    pub fn is_on_cooldown(&self, key: &HudCooldownKey) -> bool {
        self.remaining_seconds
            .get(key)
            .is_some_and(|remaining| *remaining > 0.0)
    }

    /// Convenience for the Eidolon pipeline.
    pub fn ability_on_cooldown(&self, id: &AbilityId) -> bool {
        self.is_on_cooldown(&HudCooldownKey::Ability(id.clone()))
    }
}

#[derive(Component)]
struct SpellHudRoot;

#[derive(Component, Clone)]
struct SpellHudEntry {
    /// What this entry's countdown is keyed by — `None` only for an empty slot.
    cooldown_key: Option<HudCooldownKey>,
    display_name: String,
    key_label: String,
}

pub fn spell_hud_systems(app: &mut App) {
    app.init_resource::<SpellHudState>();
    app.init_resource::<SpellHudLayoutState>();
    app.add_message::<SpellHudCooldownStarted>();
    app.add_systems(Startup, setup_spell_hud.run_if(has_client));
    app.add_systems(
        Update,
        (
            sync_spell_hud,
            // Before `update_spell_hud` counts down: the server's number wins
            // for this frame rather than the frame after.
            adopt_server_cooldowns,
            update_spell_hud,
        )
            .chain()
            .run_if(has_client)
            .run_if(in_gameplay_or_paused),
    );
    app.add_systems(
        Update,
        hide_spell_hud
            .run_if(has_client)
            .run_if(not_in_gameplay_or_paused),
    );
}

fn in_gameplay_or_paused(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

fn not_in_gameplay_or_paused(screen: Res<GameScreen>) -> bool {
    !in_gameplay_or_paused(screen)
}

fn setup_spell_hud(mut commands: Commands, theme: Res<UiTheme>) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Percent(50.0),
            padding: UiRect::all(Val::Px(10.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
        BackgroundColor(theme.panel_bg),
        SpellHudRoot,
    ));
}

/// Reads the equipped weapon's active Eidolon gesture for `slot` — its
/// `AbilityId` and the label to show — if the weapon has Eidolon abilities at
/// all. `None` means this weapon offers nothing for that slot.
///
/// The id is returned alongside the label because it is what the countdown is
/// keyed by: the gesture, not the Essence inscribed over it (an Incisione can
/// change what a slot manifests without changing its cooldown).
fn eidolon_hud_entry(
    slot: AbilitySlot,
    equipment: &Equipment,
    item_registry: &ItemRegistry,
    ability_registry: &BaseAbilityRegistry,
    essence_registry: &EssenceRegistry,
) -> Option<(AbilityId, String)> {
    let weapon = equipment.weapon.as_ref()?;
    let item = item_registry.get(&weapon.item_id)?;
    let weapon_abilities = item.ability_loadout()?;
    let ability_id = resolve_active_ability(slot, weapon_abilities, &weapon.ability_selection)?;
    let ability = ability_registry.get(ability_id)?;

    let essence_name = weapon
        .inscriptions
        .as_ref()
        .and_then(|inscriptions| inscriptions.get(slot).essence.as_ref())
        .and_then(|essence_id| essence_registry.get(essence_id))
        .map(|essence| essence.display_name().to_string());

    let label = match essence_name {
        Some(name) => format!("{} ({name})", ability.display_name()),
        None => ability.display_name().to_string(),
    };
    Some((ability_id.clone(), label))
}

#[allow(clippy::too_many_arguments)]
fn sync_spell_hud(
    mut commands: Commands,
    theme: Res<UiTheme>,
    settings: Res<GameSettingsResource>,
    mut layout_state: ResMut<SpellHudLayoutState>,
    player_query: Query<&Equipment, With<LocalPlayer>>,
    item_registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    essence_registry: Res<EssenceRegistry>,
    hud_query: Query<Entity, With<SpellHudRoot>>,
) {
    let Ok(equipment) = player_query.single() else {
        return;
    };
    let Ok(root_entity) = hud_query.single() else {
        return;
    };

    let mut signature = Vec::new();
    let mut entries = Vec::new();

    // Map each ability slot to its rebindable action and read the current
    // binding label from the settings resource so the HUD reflects rebinding.
    for (ability_slot, action) in [
        (AbilitySlot::Primary, KeyAction::CastSpellQ),
        (AbilitySlot::Secondary, KeyAction::CastSpellW),
        (AbilitySlot::Ultimate, KeyAction::CastSpellE),
    ] {
        let eidolon = eidolon_hud_entry(
            ability_slot,
            equipment,
            &item_registry,
            &ability_registry,
            &essence_registry,
        );

        let (cooldown_key, display_name) = match &eidolon {
            Some((ability_id, label)) => (
                Some(HudCooldownKey::Ability(ability_id.clone())),
                label.clone(),
            ),
            None => (None, "Empty".to_string()),
        };
        let key_label = settings.0.keybinds.get(action).label();

        signature.push((
            ability_slot,
            eidolon.as_ref().map(|(id, _)| id.clone()),
            key_label.clone(),
            display_name.clone(),
        ));
        entries.push(SpellHudEntry {
            cooldown_key,
            display_name,
            key_label,
        });
    }

    if layout_state.initialized && layout_state.signature == signature {
        return;
    }
    layout_state.initialized = true;
    layout_state.signature = signature;

    commands.entity(root_entity).despawn_related::<Children>();

    commands.entity(root_entity).with_children(|parent| {
        for entry in entries {
            parent
                .spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(theme.button_bg),
                    entry.clone(),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text(format_spell_label(&entry, 0.0)),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size),
                            ..default()
                        },
                        TextColor(theme.text_color),
                        entry,
                    ));
                });
        }
    });
}

/// Replaces the HUD's own countdown with the server `cooldown` table's.
///
/// The HUD starts a timer the moment a key is pressed, which is right up until
/// the server disagrees — a cast that was refused still greyed the key out for
/// its full duration, and a cooldown the server shortened stayed grey anyway.
/// This overwrites the local guess whenever the authoritative row moves.
///
/// Only the local player's rows are read: the table carries every entity's
/// cooldowns, and this HUD shows one character's.
fn adopt_server_cooldowns(
    mut state: ResMut<SpellHudState>,
    mut incoming: MessageReader<SpellCooldownState>,
    local_player: Query<&NetworkEntityId, With<LocalPlayer>>,
) {
    let Ok(local) = local_player.single() else {
        // Nothing to attribute the cooldowns to yet. Dropping them is right:
        // a fresh row for every live cooldown arrives with the subscription.
        incoming.clear();
        return;
    };

    for message in incoming.read() {
        if message.entity_id != local.0 {
            continue;
        }
        let key = HudCooldownKey::Ability(AbilityId::new(message.ability_id.clone()));
        if message.is_ready() {
            state.remaining_seconds.remove(&key);
        } else {
            state
                .remaining_seconds
                .insert(key, message.remaining_seconds);
        }
    }
}

fn update_spell_hud(
    time: Res<Time>,
    mut elapsed_since_label_update: Local<f32>,
    mut state: ResMut<SpellHudState>,
    mut cooldown_started: MessageReader<SpellHudCooldownStarted>,
    mut roots: Query<&mut Node, With<SpellHudRoot>>,
    mut texts: Query<(&SpellHudEntry, &mut Text)>,
) {
    let mut has_new_cooldown = false;
    for message in cooldown_started.read() {
        has_new_cooldown = true;
        state
            .remaining_seconds
            .insert(message.key.clone(), message.cooldown_seconds.max(0.0));
    }

    let delta = time.delta_secs();
    state.remaining_seconds.retain(|_, remaining| {
        *remaining = (*remaining - delta).max(0.0);
        *remaining > 0.0
    });

    if let Ok(mut root) = roots.single_mut() {
        root.display = Display::Flex;
    }

    *elapsed_since_label_update += delta;
    let should_update_labels = has_new_cooldown || *elapsed_since_label_update >= 0.1;
    if !should_update_labels {
        return;
    }
    *elapsed_since_label_update = 0.0;

    for (entry, mut text) in texts.iter_mut() {
        let remaining = entry
            .cooldown_key
            .as_ref()
            .and_then(|key| state.remaining_seconds.get(key).copied())
            .unwrap_or_default();
        let next_label = format_spell_label(entry, remaining);
        if text.0 == next_label {
            continue;
        }
        text.0 = next_label;
    }
}

fn hide_spell_hud(mut roots: Query<&mut Node, With<SpellHudRoot>>) {
    if let Ok(mut root) = roots.single_mut() {
        root.display = Display::None;
    }
}

fn format_spell_label(entry: &SpellHudEntry, remaining_seconds: f32) -> String {
    if entry.display_name == "Empty" {
        return format!("[{}] Empty", entry.key_label);
    }

    let Some(_) = entry.cooldown_key.as_ref() else {
        return format!("[{}] {}", entry.key_label, entry.display_name);
    };

    let cooldown = if remaining_seconds > 0.0 {
        format!("{remaining_seconds:.1}s")
    } else {
        "Ready".to_string()
    };

    format!(
        "[{}] {} - {}",
        entry.key_label, entry.display_name, cooldown
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ability_entry(id: &'static str, name: &str, key: &str) -> SpellHudEntry {
        SpellHudEntry {
            cooldown_key: Some(HudCooldownKey::Ability(AbilityId::new(id))),
            display_name: name.to_string(),
            key_label: key.to_string(),
        }
    }

    #[test]
    fn spell_label_formats_ready_cooldown_and_empty_states() {
        let entry = ability_entry("test", "Test Ability", "Q");
        let empty_entry = SpellHudEntry {
            cooldown_key: None,
            display_name: "Empty".to_string(),
            key_label: "W".to_string(),
        };

        assert_eq!(format_spell_label(&entry, 0.0), "[Q] Test Ability - Ready");
        assert_eq!(format_spell_label(&entry, 1.25), "[Q] Test Ability - 1.2s");
        assert_eq!(format_spell_label(&empty_entry, 0.0), "[W] Empty");
    }

    #[test]
    fn eidolon_gesture_counts_down_like_a_spell() {
        let entry = SpellHudEntry {
            cooldown_key: Some(HudCooldownKey::Ability(AbilityId::new("staff_bolt"))),
            display_name: "Getto".to_string(),
            key_label: "Q".to_string(),
        };
        assert_eq!(format_spell_label(&entry, 2.5), "[Q] Getto - 2.5s");
    }

    #[test]
    fn ability_cooldown_tracking_works() {
        let mut state = SpellHudState::default();
        let id = AbilityId::new("bolt");
        assert!(!state.ability_on_cooldown(&id));

        state
            .remaining_seconds
            .insert(HudCooldownKey::Ability(id.clone()), 3.0);
        assert!(state.ability_on_cooldown(&id));
    }
}
