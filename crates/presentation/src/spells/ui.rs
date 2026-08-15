//! Client-only modular spell HUD.
//!
//! UI isolata e data-driven per mostrare spell/keybind/cooldown.
//! Per rimuoverla basta togliere `spell_hud_systems(app)` dal `SpellsPlugin`
//! e cancellare questo file.

use bevy::prelude::*;
use std::collections::HashMap;

use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::abilities::{
    resolve_active_ability, AbilityId, AbilitySlot, BaseAbilityRegistry, EssenceRegistry,
};
use bevymmo_shared::items::components::Equipment;
use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::movement::MoveTarget;
use bevymmo_shared::network::mode::has_client;
use bevymmo_shared::network::protocol::{
    Channel2, LookDirection, NetworkEntityId, Position, SpellCastCommand,
};
use bevymmo_shared::spells::{HotbarSlot, SpellHotbar, SpellId};
use bevymmo_shared::targeting::CurrentTarget;
use bevymmo_shared::user_settings::{GameSettingsResource, KeyAction};
use bevymmo_shared::entity::LocalPlayer;
use lightyear::prelude::MessageSender;

use crate::game_state::{GameScreen, Screen};
use crate::spells::cursor::{cursor_ground_point, flat_direction_towards};
use crate::spells::input::stops_movement_for_cast;
use crate::ui::theme::UiTheme;

/// What a HUD cooldown countdown is keyed by.
///
/// The two cast pipelines name what they fire differently: the classic hotbar
/// sends a `SpellId`, while an Eidolon weapon sends a *slot* that resolves to
/// an `AbilityId` (the gesture) — see `crate::spells::eidolon_input`. Keying
/// the countdown by the union of the two lets one timer and one label path
/// serve both. Before this existed the HUD tracked spells only, so equipping
/// an Eidolon weapon made the cooldown disappear from Q/W/E entirely: those
/// entries carry no `SpellId` at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HudCooldownKey {
    Spell(SpellId),
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
    /// `(slot, spell_id, key_label, display_name)` — `display_name` is
    /// included (not derivable from `spell_id` alone) so the HUD also
    /// rebuilds when an Eidolon weapon's Incisione changes, even though
    /// `spell_id` stays `None` throughout that case.
    signature: Vec<(HotbarSlot, Option<SpellId>, String, String)>,
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

    /// Convenience for the classic hotbar pipeline.
    pub fn spell_on_cooldown(&self, id: &SpellId) -> bool {
        self.is_on_cooldown(&HudCooldownKey::Spell(id.clone()))
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
    /// Set only for classic hotbar spells. An Eidolon gesture is cast by
    /// *slot*, not by id, so `cast_spell_from_hud_click` deliberately no-ops
    /// when this is `None`.
    spell_id: Option<SpellId>,
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
        (sync_spell_hud, cast_spell_from_hud_click, update_spell_hud)
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
/// all. `None` means this weapon uses the classic `SpellHotbar` model instead
/// and `sync_spell_hud` falls back to that.
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
    let weapon_abilities = item.weapon_abilities()?;
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
    registry: Res<bevymmo_shared::spells::SpellRegistry>,
    settings: Res<GameSettingsResource>,
    mut layout_state: ResMut<SpellHudLayoutState>,
    player_query: Query<(&SpellHotbar, &Equipment), With<LocalPlayer>>,
    item_registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    essence_registry: Res<EssenceRegistry>,
    hud_query: Query<Entity, With<SpellHudRoot>>,
) {
    let Ok((hotbar, equipment)) = player_query.single() else {
        return;
    };
    let Ok(root_entity) = hud_query.single() else {
        return;
    };

    let mut signature = Vec::new();
    let mut entries = Vec::new();

    // Map each hotbar slot to its rebindable action and read the current
    // binding label from the settings resource so the HUD reflects rebinding.
    for (slot, ability_slot, action) in [
        (HotbarSlot::Q, AbilitySlot::Primary, KeyAction::CastSpellQ),
        (HotbarSlot::W, AbilitySlot::Secondary, KeyAction::CastSpellW),
        (HotbarSlot::E, AbilitySlot::Ultimate, KeyAction::CastSpellE),
    ] {
        let eidolon = eidolon_hud_entry(
            ability_slot,
            equipment,
            &item_registry,
            &ability_registry,
            &essence_registry,
        );

        // An Eidolon weapon's gesture isn't a `SpellId` at all — leave it
        // `None` so `cast_spell_from_hud_click` (which only acts when
        // `spell_id` is `Some`) safely no-ops on this entry.
        let spell_id = if eidolon.is_some() {
            None
        } else {
            hotbar.spell_for_slot(slot).cloned()
        };
        let (cooldown_key, display_name) = match eidolon {
            Some((ability_id, label)) => (Some(HudCooldownKey::Ability(ability_id)), label),
            None => {
                let label = spell_id
                    .as_ref()
                    .and_then(|id| registry.get(id))
                    .map(|spell_def| spell_def.display_name().to_string())
                    .unwrap_or_else(|| "Empty".to_string());
                let key = spell_id.clone().map(HudCooldownKey::Spell);
                (key, label)
            }
        };
        let key_label = settings.0.keybinds.get(action).label();

        signature.push((
            slot,
            spell_id.clone(),
            key_label.clone(),
            display_name.clone(),
        ));
        entries.push(SpellHudEntry {
            spell_id,
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
                    Button,
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

fn cast_spell_from_hud_click(
    interactions: Query<(&Interaction, &SpellHudEntry), (Changed<Interaction>, With<Button>)>,
    hud_state: Res<SpellHudState>,
    current_target: Res<CurrentTarget>,
    target_ids: Query<&NetworkEntityId>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut controlled_players: Query<(&Position, &mut LookDirection), With<LocalPlayer>>,
    mut move_target: ResMut<MoveTarget>,
    mut senders: Query<&mut MessageSender<SpellCastCommand>, With<ConnectedClient>>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
    registry: Res<bevymmo_shared::spells::SpellRegistry>,
) {
    for (interaction, entry) in interactions.iter() {
        let Some(spell_id) = &entry.spell_id else {
            continue;
        };
        if *interaction != Interaction::Pressed || hud_state.spell_on_cooldown(spell_id) {
            continue;
        }

        let target_position = cursor_ground_point(&windows, &cameras);

        let mut target_id = None;
        if let Some(target_entity) = current_target.entity {
            if let Ok(net_id) = target_ids.get(target_entity) {
                target_id = Some(net_id.0);
            }
        }

        for mut sender in senders.iter_mut() {
            sender.send::<Channel2>(SpellCastCommand {
                spell_id: spell_id.as_str().to_owned(),
                target_position,
                target_id,
            });
        }

        if let Some(spell_def) = registry.get(spell_id) {
            if let Ok((player_position, mut look_direction)) = controlled_players.single_mut() {
                let face_direction = target_position
                    .and_then(|target| flat_direction_towards(player_position.0, target));
                if let Some(direction) = face_direction {
                    look_direction.0 = direction;
                }
                if stops_movement_for_cast(
                    spell_def.cast_kind(),
                    spell_def.config().channel_movement,
                ) {
                    move_target.0 = None;
                }
            }

            if spell_def.cast_kind() == bevymmo_shared::spells::CastKind::Instant {
                hud_cooldowns.write(SpellHudCooldownStarted {
                    key: HudCooldownKey::Spell(spell_id.clone()),
                    cooldown_seconds: spell_def.config().cooldown_seconds,
                });
            }
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

    // Note this keys off `cooldown_key`, not `spell_id`. An Eidolon gesture
    // deliberately has no `SpellId` (so `cast_spell_from_hud_click` no-ops on
    // it) yet still has a cooldown, keyed by `AbilityId` — branching on
    // `spell_id` here is exactly what used to drop the countdown for every
    // equipped Eidolon weapon.
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

    fn spell_entry(id: &'static str, name: &str, key: &str) -> SpellHudEntry {
        SpellHudEntry {
            spell_id: Some(SpellId::new(id)),
            cooldown_key: Some(HudCooldownKey::Spell(SpellId::new(id))),
            display_name: name.to_string(),
            key_label: key.to_string(),
        }
    }

    #[test]
    fn spell_label_formats_ready_cooldown_and_empty_states() {
        let entry = spell_entry("test", "Test Spell", "Q");
        let empty_entry = SpellHudEntry {
            spell_id: None,
            cooldown_key: None,
            display_name: "Empty".to_string(),
            key_label: "W".to_string(),
        };

        assert_eq!(format_spell_label(&entry, 0.0), "[Q] Test Spell - Ready");
        assert_eq!(format_spell_label(&entry, 1.25), "[Q] Test Spell - 1.2s");
        assert_eq!(format_spell_label(&empty_entry, 0.0), "[W] Empty");
    }

    /// An Eidolon gesture carries no `SpellId` — the label must still name it.
    #[test]
    fn spell_label_shows_the_eidolon_gesture_despite_a_none_spell_id() {
        let entry = SpellHudEntry {
            spell_id: None,
            cooldown_key: Some(HudCooldownKey::Ability(AbilityId::new("staff_bolt"))),
            display_name: "Getto (Fuoco)".to_string(),
            key_label: "Q".to_string(),
        };
        assert_eq!(format_spell_label(&entry, 0.0), "[Q] Getto (Fuoco) - Ready");
    }

    /// The regression: equipping an Eidolon weapon made the cooldown vanish
    /// from Q/W/E, because the countdown was keyed by `SpellId` and those
    /// entries have none. It is keyed by the gesture's `AbilityId` instead.
    #[test]
    fn eidolon_gesture_counts_down_like_a_spell() {
        let entry = SpellHudEntry {
            spell_id: None,
            cooldown_key: Some(HudCooldownKey::Ability(AbilityId::new("staff_bolt"))),
            display_name: "Getto".to_string(),
            key_label: "Q".to_string(),
        };
        assert_eq!(format_spell_label(&entry, 2.5), "[Q] Getto - 2.5s");
    }

    /// A gesture and a spell that happen to share an id string are distinct
    /// countdowns — the key carries which pipeline it came from.
    #[test]
    fn spell_and_ability_keys_with_the_same_id_do_not_collide() {
        let mut state = SpellHudState::default();
        state
            .remaining_seconds
            .insert(HudCooldownKey::Spell(SpellId::new("bolt")), 3.0);

        assert!(state.spell_on_cooldown(&SpellId::new("bolt")));
        assert!(!state.ability_on_cooldown(&AbilityId::new("bolt")));
    }
}
