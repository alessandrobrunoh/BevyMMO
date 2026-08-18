//! Client-only modular ability hotbar.
//!
//! Data-driven UI showing all active ability slots across weapon, helmet,
//! chestplate, and shoes in a compact 3-row grid (key / name / cooldown).
//! Entries are derived from equipped items via `resolve_active_ability`.

use bevy::prelude::*;
use std::collections::HashMap;

use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::server_feed::SpellCooldownState;
use bevymmo_client::user_settings::{GameSettingsResource, KeyAction};
use bevymmo_gameplay::abilities::{
    resolve_active_ability, AbilityId, AbilitySlot, BaseAbilityRegistry,
};
use bevymmo_gameplay::items::components::Equipment;
use bevymmo_gameplay::items::registry::ItemRegistry;
use bevymmo_network::network::mode::has_client;
use bevymmo_network::network::protocol::NetworkEntityId;

use crate::game_state::{GameScreen, Screen};
use crate::ui::theme::UiTheme;

/// What a HUD cooldown countdown is keyed by.
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

/// Describes one hotbar column read from equipment + settings.
#[derive(Component, Clone)]
struct SpellHudEntry {
    /// What this entry's countdown is keyed by — `None` for empty slots.
    cooldown_key: Option<HudCooldownKey>,
    display_name: String,
    key_label: String,
}

#[derive(Resource, Default)]
struct SpellHudLayoutState {
    initialized: bool,
    /// `(ability_slot, ability_id, key_label, display_name)` — rebuilds when
    /// any of these change (weapon swap, gear change, Incisione rewrite).
    signature: Vec<(AbilitySlot, Option<AbilityId>, String, String)>,
}

impl SpellHudState {
    pub fn is_on_cooldown(&self, key: &HudCooldownKey) -> bool {
        self.remaining_seconds
            .get(key)
            .is_some_and(|remaining| *remaining > 0.0)
    }

    pub fn ability_on_cooldown(&self, id: &AbilityId) -> bool {
        self.is_on_cooldown(&HudCooldownKey::Ability(id.clone()))
    }
}

#[derive(Component)]
struct SpellHudRoot;

pub fn spell_hud_systems(app: &mut App) {
    app.init_resource::<SpellHudState>();
    app.init_resource::<SpellHudLayoutState>();
    app.add_message::<SpellHudCooldownStarted>();
    app.add_systems(Startup, setup_spell_hud.run_if(has_client));
    app.add_systems(
        Update,
        (
            sync_spell_hud,
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
            bottom: Val::Px(86.0),
            left: Val::Percent(2.0),
            width: Val::Percent(96.0),
            padding: UiRect::all(Val::Px(8.0)),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(6.0),
            overflow: Overflow::clip_x(),
            ..default()
        },
        BackgroundColor(theme.panel_bg),
        SpellHudRoot,
    ));
}

/// A single logical hotbar slot: which piece of equipment and which ability
/// slot within it, plus the `KeyAction` used to read the current binding label.
struct HotbarSlotDef {

    action: KeyAction,
    slot: AbilitySlot,
    /// Extracts the relevant `ItemInstance` from `Equipment`.
    equip_fn: fn(&Equipment) -> &Option<bevymmo_gameplay::items::instance::ItemInstance>,
}

/// All 6 hotbar columns in display order: three weapon slots and one active
/// ability for each armor piece.
const HOTBAR_SLOTS: [HotbarSlotDef; 6] = [
    HotbarSlotDef { action: KeyAction::CastPrimary,          slot: AbilitySlot::Primary,   equip_fn: |e| &e.weapon },
    HotbarSlotDef { action: KeyAction::CastSecondary,        slot: AbilitySlot::Secondary, equip_fn: |e| &e.weapon },
    HotbarSlotDef { action: KeyAction::CastUltimate,         slot: AbilitySlot::Ultimate,  equip_fn: |e| &e.weapon },
    HotbarSlotDef { action: KeyAction::CastHelmet,     slot: AbilitySlot::Primary, equip_fn: |e| &e.helmet },
    HotbarSlotDef { action: KeyAction::CastChestplate, slot: AbilitySlot::Primary, equip_fn: |e| &e.armor },
    HotbarSlotDef { action: KeyAction::CastBoots,      slot: AbilitySlot::Primary, equip_fn: |e| &e.shoes },
];

/// Resolves the active ability for one equipped item + ability-slot pair.
///
/// Returns `(AbilityId, display_name)` if the item exists, has an ability
/// loadout, and a valid ability can be resolved through its selection.
fn resolve_equipment_entry(
    equipped: &Option<bevymmo_gameplay::items::instance::ItemInstance>,
    slot: AbilitySlot,
    item_registry: &ItemRegistry,
    ability_registry: &BaseAbilityRegistry,
) -> Option<(AbilityId, String)> {
    let instance = equipped.as_ref()?;
    let item = item_registry.get(&instance.item_id)?;
    let loadout = item.ability_loadout()?;
    let ability_id = if matches!(
        item.config().category,
        bevymmo_gameplay::items::definition::ItemCategory::Armor
    ) {
        bevymmo_gameplay::abilities::resolve_armor_ability(loadout, &instance.ability_selection)?
    } else {
        resolve_active_ability(slot, loadout, &instance.ability_selection)?
    };
    let ability = ability_registry.get(ability_id)?;
    Some((ability_id.clone(), ability.display_name().to_string()))
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

    for def in &HOTBAR_SLOTS {
        let resolved = resolve_equipment_entry(
            (def.equip_fn)(equipment),
            def.slot,
            &item_registry,
            &ability_registry,
        );

        let (cooldown_key, display_name) = match &resolved {
            Some((id, name)) => (
                Some(HudCooldownKey::Ability(id.clone())),
                name.clone(),
            ),
            None => (None, "Empty".to_string()),
        };
        let key_label = display_key_label(&settings.0.keybinds.get(def.action).label());

        signature.push((
            def.slot,
            resolved.as_ref().map(|(id, _)| id.clone()),
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
                        width: Val::Percent(10.0),
                        min_width: Val::Px(0.0),
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                        overflow: Overflow::clip_x(),
                        ..default()
                    },
                    BackgroundColor(theme.button_bg),
                    entry.clone(),
                ))
                .with_children(|col| {
                    // Row 1 — key label
                    col.spawn((
                        Text(entry.key_label.clone()),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size),
                            ..default()
                        },
                        TextColor(theme.text_color),
                    ));
                    // Row 2 — ability name
                    col.spawn((
                        Text(if entry.display_name == "Empty" {
                            "Empty".into()
                        } else {
                            entry.display_name.clone()
                        }),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size - 2.0),
                            ..default()
                        },
                        TextColor(theme.text_color),
                        Name::new("hotbar-name"),
                    ));
                    // Row 3 — cooldown placeholder
                    col.spawn((
                        Text("—".into()),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size - 2.0),
                            ..default()
                        },
                        TextColor(theme.text_color),
                        Name::new("hotbar-cooldown"),
                        entry.clone(),
                    ));
                });
        }
    });
}

/// Overwrites local cooldown guesses with authoritative server values.
fn adopt_server_cooldowns(
    mut state: ResMut<SpellHudState>,
    mut incoming: MessageReader<SpellCooldownState>,
    local_player: Query<&NetworkEntityId, With<LocalPlayer>>,
) {
    let Ok(local) = local_player.single() else {
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
    mut texts: Query<(&SpellHudEntry, &mut Text), With<Name>>,
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

    // Only update the cooldown row (Name = "hotbar-cooldown").
    for (entry, mut text) in texts.iter_mut() {
        let remaining = entry
            .cooldown_key
            .as_ref()
            .and_then(|key| state.remaining_seconds.get(key).copied())
            .unwrap_or_default();
        let next = format_cooldown_text(entry, remaining);
        if text.0 == next {
            continue;
        }
        text.0 = next;
    }
}

fn hide_spell_hud(mut roots: Query<&mut Node, With<SpellHudRoot>>) {
    if let Ok(mut root) = roots.single_mut() {
        root.display = Display::None;
    }
}

/// Formats the third row of a hotbar cell.
fn display_key_label(label: &str) -> String {
    label
        .strip_prefix("Digit")
        .or_else(|| label.strip_prefix("Key"))
        .unwrap_or(label)
        .to_string()
}

fn format_cooldown_text(entry: &SpellHudEntry, remaining_seconds: f32) -> String {
    if entry.cooldown_key.is_none() || entry.display_name == "Empty" {
        return "—".to_string();
    }
    if remaining_seconds > 0.0 {
        format!("{remaining_seconds:.1}s")
    } else {
        "Ready".to_string()
    }
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

    fn empty_entry(key: &str) -> SpellHudEntry {
        SpellHudEntry {
            cooldown_key: None,
            display_name: "Empty".to_string(),
            key_label: key.to_string(),
        }
    }

    #[test]
    fn technical_key_names_become_player_facing_labels() {
        assert_eq!(display_key_label("Digit1"), "1");
        assert_eq!(display_key_label("KeyD"), "D");
        assert_eq!(display_key_label("PageUp"), "PageUp");
    }

    #[test]
    fn cooldown_text_formats_all_states() {
        let entry = ability_entry("bolt", "Arcane Bolt", "1");
        assert_eq!(format_cooldown_text(&entry, 0.0), "Ready");
        assert_eq!(format_cooldown_text(&entry, 2.5), "2.5s");
        assert_eq!(format_cooldown_text(&entry, 0.09), "0.1s");
    }

    #[test]
    fn empty_slot_shashes() {
        let entry = empty_entry("D");
        assert_eq!(format_cooldown_text(&entry, 0.0), "—");
        assert_eq!(format_cooldown_text(&entry, 99.0), "—");
    }

    #[test]
    fn nine_hotbar_slots_defined() {
        assert_eq!(HOTBAR_SLOTS.len(), 6);
        // Weapon 3 + one active ability per armor piece.
        assert_eq!(HOTBAR_SLOTS[0].action, KeyAction::CastPrimary);
        assert_eq!(HOTBAR_SLOTS[2].action, KeyAction::CastUltimate);
        assert_eq!(HOTBAR_SLOTS[3].action, KeyAction::CastHelmet);
        assert_eq!(HOTBAR_SLOTS[4].action, KeyAction::CastChestplate);
        assert_eq!(HOTBAR_SLOTS[5].action, KeyAction::CastBoots);
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

    #[test]
    fn hud_cooldown_key_equality() {
        let a = HudCooldownKey::Ability(AbilityId::new("fireball"));
        let b = HudCooldownKey::Ability(AbilityId::new("fireball"));
        let c = HudCooldownKey::Ability(AbilityId::new("icebolt"));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
