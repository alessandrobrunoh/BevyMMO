//! Client input for the Eidolon cast pipeline.
//!
//! Routes Q/W/E to `EidolonCastCommand` instead of `SpellCastCommand`
//! whenever the equipped weapon has Eidolon gestures
//! (`Item::weapon_abilities()`), so the two pipelines never both fire for
//! the same key press — see the matching early-bail in
//! `crate::spells::input::cast_spells_on_key`.
//!
//! **Il cast parte al RILASCIO del tasto, non alla pressione.** La pressione
//! apre una finestra di mira ([`AbilityAim`]) durante la quale
//! [`crate::spells::aim_preview`] disegna a terra l'area esatta che il gesto
//! colpirà; il rilascio la chiude e spedisce il comando. Un tap veloce si
//! comporta come prima, perché pressione e rilascio arrivano a pochi frame di
//! distanza.
//!
//! Instant-only for now: no CastTime/Channeling equivalent exists yet for
//! Eidolon abilities, so there is no movement-freeze/cast-bar handling here.

use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::abilities::{
    resolve_active_ability, AbilityAim, AbilityId, AbilitySlot, BaseAbilityRegistry,
};
use bevymmo_shared::items::components::Equipment;
use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::network::protocol::{
    Channel2, EidolonCastCommand, LookDirection, NetworkEntityId, Position,
};
use bevymmo_shared::targeting::CurrentTarget;
use bevymmo_shared::user_settings::{GameSettingsResource, KeyAction};
use lightyear::prelude::Controlled;
use lightyear::prelude::MessageSender;

use crate::game_state::{GameScreen, Screen};
use crate::spells::cursor::{cursor_ground_point, flat_direction_towards};
use crate::spells::ui::{HudCooldownKey, SpellHudCooldownStarted, SpellHudState};

/// Tasto ↔ slot. Vive qui e non su `AbilitySlot` di proposito: il legame fra
/// tasto fisico e ruolo di gameplay è un dettaglio di input, vedi il commento
/// in `bevymmo_shared::abilities::slot`.
pub const SLOT_BINDINGS: [(KeyAction, AbilitySlot); 3] = [
    (KeyAction::CastSpellQ, AbilitySlot::Primary),
    (KeyAction::CastSpellW, AbilitySlot::Secondary),
    (KeyAction::CastSpellE, AbilitySlot::Ultimate),
];

#[allow(clippy::too_many_arguments)]
pub fn cast_eidolon_abilities_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    settings: Res<GameSettingsResource>,
    screen: Res<GameScreen>,
    current_target: Res<CurrentTarget>,
    mut aim: ResMut<AbilityAim>,
    target_ids: Query<&NetworkEntityId>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut controlled_players: Query<(&Equipment, &Position, &mut LookDirection), With<Controlled>>,
    mut cast_senders: Query<&mut MessageSender<EidolonCastCommand>, With<ConnectedClient>>,
    registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    hud_state: Res<SpellHudState>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
) {
    // Qualunque cosa faccia cadere il contesto di gioco — schermata diversa,
    // player sparito, arma non-Eidolon in mano — chiude la mira. Lasciarla
    // aperta significherebbe lanciare al rilascio di un tasto che il
    // giocatore non sta più usando per mirare.
    let Some(keys) = keys else {
        aim.clear();
        return;
    };
    if !matches!(screen.0, Screen::InGame | Screen::Paused) {
        aim.clear();
        return;
    }

    let Ok((equipment, player_position, mut look_direction)) = controlled_players.single_mut()
    else {
        aim.clear();
        return;
    };

    // Only these keys when the equipped weapon actually has Eidolon
    // gestures — `cast_spells_on_key` owns them otherwise.
    let Some(weapon) = &equipment.weapon else {
        aim.clear();
        return;
    };
    let Some(item) = registry.get(&weapon.item_id) else {
        aim.clear();
        return;
    };
    let Some(weapon_abilities) = item.weapon_abilities() else {
        aim.clear();
        return;
    };

    let target_position = cursor_ground_point(&windows, &cameras);

    let target_id = current_target
        .entity
        .and_then(|entity| target_ids.get(entity).ok())
        .map(|net_id| net_id.0);

    // La pressione apre la mira. `just_pressed` (modificatori compresi) resta
    // il gesto d'apertura; il rilascio invece li ignora — vedi
    // `GameSettingsResource::just_released`.
    for (action, slot) in SLOT_BINDINGS {
        if settings.just_pressed(action, &keys) {
            aim.begin(slot);
        }
    }

    // Mentre si mira il personaggio segue il cursore ogni frame, non solo
    // all'istante del lancio: l'anteprima di un cono è disegnata attorno a
    // `LookDirection`, quindi senza questo resterebbe ferma mentre il mouse
    // si muove. Il server ricalcola comunque il facing da `target_position`
    // quando riceve il comando, quindi resta predizione cosmetica come già in
    // `cast_spells_on_key`.
    if aim.slot.is_some() {
        aim.ground_point = target_position;
        if let Some(direction) =
            target_position.and_then(|target| flat_direction_towards(player_position.0, target))
        {
            look_direction.0 = direction;
        }
    }

    // Il rilascio chiude la mira e, se non è stata annullata con Esc, spedisce
    // il cast.
    for (action, slot) in SLOT_BINDINGS {
        if aim.slot != Some(slot) || !settings.just_released(action, &keys) {
            continue;
        }

        let cancelled = aim.cancelled;
        aim.clear();
        if cancelled {
            continue;
        }

        // Resolve the gesture this slot actually fires, so the HUD countdown is
        // keyed by the same `AbilityId` the server puts on cooldown. Without
        // this the Eidolon entries never got a countdown at all, and Q/W/E
        // looked like they had no cooldown once an Eidolon weapon was equipped.
        let ability = active_ability(slot, weapon_abilities, weapon, &ability_registry);

        // Predicted locally, exactly like the classic spell path: the server
        // is still authoritative and only starts the real cooldown on a cast it
        // accepts, but gating here stops the key from spamming the channel
        // while the countdown runs. Il controllo sta sul RILASCIO, non sulla
        // pressione: così il cooldown che scade mentre si tiene premuto lascia
        // comunque partire il gesto invece di richiedere una ri-pressione.
        if let Some((ability_id, _)) = &ability {
            if hud_state.ability_on_cooldown(ability_id) {
                continue;
            }
        }

        for mut sender in cast_senders.iter_mut() {
            sender.send::<Channel2>(EidolonCastCommand {
                slot,
                target_position,
                target_id,
            });
        }

        if let Some((ability_id, cooldown_seconds)) = ability {
            hud_cooldowns.write(SpellHudCooldownStarted {
                key: HudCooldownKey::Ability(ability_id),
                cooldown_seconds,
            });
        }
    }
}

/// Gesto attivo sullo slot, con il suo cooldown base. `None` se l'arma non
/// offre nulla per quello slot o l'id inciso non è più nel registry.
fn active_ability(
    slot: AbilitySlot,
    weapon_abilities: &bevymmo_shared::abilities::WeaponAbilities,
    weapon: &bevymmo_shared::items::instance::ItemInstance,
    ability_registry: &BaseAbilityRegistry,
) -> Option<(AbilityId, f32)> {
    let ability_id = resolve_active_ability(slot, weapon_abilities, &weapon.ability_selection)?;
    let ability = ability_registry.get(ability_id)?;
    Some((ability_id.clone(), ability.base_params().cooldown))
}
