//! Anteprima a terra dell'area che colpirà il gesto in mira.
//!
//! Disegnata finché il tasto dell'abilità resta premuto (vedi
//! [`crate::spells::eidolon_input`]), a gizmos: sono ridisegnati ogni frame,
//! quindi non c'è nessuna entità/mesh/materiale da tenere in vita e sparire
//! è gratis.
//!
//! Il punto centrale del modulo è che **non calcola nulla di suo**: centro,
//! raggio e forma arrivano da `BaseAbility::impact_center` / `impact_radius` /
//! `impact_shape`, le identiche funzioni che il server chiamerà un istante
//! dopo per applicare l'effetto. Finché resta così, l'anteprima non può
//! mentire su dove cadrà il colpo.

use bevymmo_domain::EntityId;
use std::f32::consts::TAU;

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevymmo_gameplay::abilities::base_ability::FORWARD_LANE_HALF_WIDTH;
use bevymmo_gameplay::abilities::{
    resolve_slot_preview, AbilityAim, AbilityGeometry, BaseAbilityRegistry, KnownGlyphs,
    ModifierRegistry, SlotPreview,
};
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_gameplay::items::components::Equipment;
use bevymmo_gameplay::items::registry::ItemRegistry;
use bevymmo_network::network::protocol::{LookDirection, Position};
use bevymmo_gameplay::spells::context::{AoeShape, SpellCastContext};
use bevymmo_gameplay::stats::components::CombatStats;
use bevymmo_client::user_settings::{GameSettingsResource, KeyAction};

use crate::spells::ui::SpellHudState;

/// Area effettivamente colpita.
const IMPACT_COLOR: Srgba = css::AQUA;
/// Gittata massima entro cui il gesto può essere piazzato.
const RANGE_COLOR: Srgba = css::DIM_GRAY;
/// Lo slot non partirà: Glifo sconosciuto o cooldown in corso.
const BLOCKED_COLOR: Srgba = css::ORANGE_RED;

/// Quanto stacca da terra il disegno, per non finire in z-fighting col
/// terreno.
const GROUND_OFFSET: f32 = 0.05;
/// Segmenti dell'arco di un cono. 32 su 360° è abbastanza fitto da non
/// mostrare spigoli alle aperture tipiche (60-90°).
const CONE_ARC_SEGMENTS: usize = 32;

/// Disegna l'anteprima del gesto attualmente in mira.
///
/// No-op se non si sta mirando, quindi il costo a riposo è una lettura di
/// risorsa.
#[allow(clippy::too_many_arguments)]
pub fn draw_ability_aim_preview(
    mut gizmos: Gizmos,
    aim: Res<AbilityAim>,
    players: Query<(&Equipment, &KnownGlyphs, &Position, &LookDirection), With<LocalPlayer>>,
    item_registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    modifier_registry: Res<ModifierRegistry>,
    hud_state: Res<SpellHudState>,
) {
    let Some(slot) = aim.slot else {
        return;
    };
    if aim.cancelled {
        return;
    }

    let Ok((equipment, known, position, look_direction)) = players.single() else {
        return;
    };
    let Some(weapon) = &equipment.weapon else {
        return;
    };
    let Some(item) = item_registry.get(&weapon.item_id) else {
        return;
    };
    let Some(weapon_abilities) = item.weapon_abilities() else {
        return;
    };
    let inscriptions = weapon.inscriptions.clone().unwrap_or_default();

    let preview = resolve_slot_preview(
        slot,
        weapon_abilities,
        &weapon.ability_selection,
        &inscriptions,
        known,
        &ability_registry,
        &modifier_registry,
    );

    // Slot bloccato (Glifo sconosciuto, o dati incoerenti): niente forma da
    // disegnare, ma un cerchietto rosso ai piedi del personaggio è meglio del
    // silenzio — dice "questo tasto non farà nulla" prima del rilascio.
    let Ok(SlotPreview { ability, params }) = preview else {
        draw_flat_circle(&mut gizmos, position.0, 1.0, BLOCKED_COLOR);
        return;
    };

    let on_cooldown = hud_state.ability_on_cooldown(&ability.id());
    let color = if on_cooldown {
        BLOCKED_COLOR
    } else {
        IMPACT_COLOR
    };

    // `impact_center`/`impact_shape` leggono il contesto di cast: lo si
    // costruisce identico a quello che il server costruirà, con il punto di
    // mira corrente al posto di `target_position`. `potential_targets` resta
    // vuoto perché nessuna delle tre funzioni geometriche lo consulta.
    let combat = CombatStats {
        attack_power: 0.0,
        armor: 0.0,
    };
    let ctx = SpellCastContext::new(
        EntityId::PLACEHOLDER,
        position.0,
        &combat,
        look_direction.0,
        aim.ground_point,
        None,
        &[],
    );

    let center = ability.impact_center(&params, &ctx);
    let radius = ability.impact_radius(&params);

    match ability.geometry() {
        AbilityGeometry::Circle { .. } => {
            // La gittata è ciò che limita dove il cerchio può essere piazzato:
            // mostrarla spiega perché l'area smette di seguire il cursore
            // quando si punta troppo lontano.
            if params.range > 0.0 {
                draw_flat_circle(&mut gizmos, position.0, params.range, RANGE_COLOR);
            }
            draw_flat_circle(&mut gizmos, center, radius, color);
        }
        AbilityGeometry::Cone { .. } => {
            if let AoeShape::Cone {
                direction,
                angle_deg,
            } = ability.impact_shape(&ctx)
            {
                draw_flat_cone(&mut gizmos, center, radius, direction, angle_deg, color);
            }
        }
        AbilityGeometry::Projectile { .. } => {
            draw_forward_lane(&mut gizmos, position.0, look_direction.0, params.range, color);
        }
        AbilityGeometry::SelfBuff { .. } => {
            draw_flat_circle(&mut gizmos, position.0, 1.0, color);
        }
    }
}

/// Annulla la mira in corso con `Esc`, **consumando** la pressione.
///
/// `Escape` è il default sia di `TogglePause` sia di `ClearTarget`: senza
/// consumarlo, annullare un gesto aprirebbe anche il menù di pausa e
/// deselezionerebbe il bersaglio. Il sistema è quindi ordinato prima di
/// entrambi i consumatori — vedi la registrazione in
/// [`crate::spells::SpellsHudPlugin`].
pub fn cancel_ability_aim_on_escape(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    mut aim: ResMut<AbilityAim>,
) {
    if aim.slot.is_none() || aim.cancelled {
        return;
    }
    if !settings.just_pressed(KeyAction::TogglePause, &keys) {
        return;
    }

    // Lo slot resta occupato finché il tasto non viene rilasciato: azzerarlo
    // qui farebbe ripartire la mira da sola al frame dopo, visto che il tasto
    // dell'abilità è ancora premuto.
    aim.cancelled = true;
    settings.consume_press(KeyAction::TogglePause, &mut keys);
}

/// Cerchio orizzontale, disegnato appena sopra il terreno.
fn draw_flat_circle(gizmos: &mut Gizmos, center: Vec3, radius: f32, color: Srgba) {
    if radius <= 0.0 {
        return;
    }
    gizmos.circle(
        Isometry3d::new(
            center + Vec3::Y * GROUND_OFFSET,
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        ),
        radius,
        color,
    );
}

/// Settore circolare: l'arco alla gittata più i due lati che tornano
/// all'apice, così si legge sia quanto è lungo sia quanto è largo.
fn draw_flat_cone(
    gizmos: &mut Gizmos,
    apex: Vec3,
    radius: f32,
    direction: Vec3,
    angle_deg: f32,
    color: Srgba,
) {
    let axis = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
    if radius <= 0.0 || axis == Vec3::ZERO {
        return;
    }

    // Un cono >= 360° è un cerchio: l'arco si chiuderebbe su se stesso e i
    // due lati sarebbero rumore.
    if angle_deg >= 360.0 {
        draw_flat_circle(gizmos, apex, radius, color);
        return;
    }

    let apex = apex + Vec3::Y * GROUND_OFFSET;
    let half_angle = angle_deg.to_radians() / 2.0;

    let point_at = |offset: f32| {
        let rotation = Quat::from_rotation_y(offset);
        apex + rotation * axis * radius
    };

    let steps = (CONE_ARC_SEGMENTS as f32 * (half_angle * 2.0 / TAU))
        .ceil()
        .max(2.0) as usize;
    let mut previous = point_at(-half_angle);
    gizmos.line(apex, previous, color);
    for step in 1..=steps {
        let offset = -half_angle + half_angle * 2.0 * (step as f32 / steps as f32);
        let current = point_at(offset);
        gizmos.line(previous, current, color);
        previous = current;
    }
    gizmos.line(previous, apex, color);
}

/// Corridoio frontale entro cui un gesto `Projectile` aggancia da solo il
/// primo bersaglio (`BaseAbility::projectile_target`): asse centrale più i due
/// bordi a `FORWARD_LANE_HALF_WIDTH`.
fn draw_forward_lane(
    gizmos: &mut Gizmos,
    origin: Vec3,
    look_direction: Vec3,
    range: f32,
    color: Srgba,
) {
    let forward = Vec3::new(look_direction.x, 0.0, look_direction.z).normalize_or_zero();
    if range <= 0.0 || forward == Vec3::ZERO {
        return;
    }

    let origin = origin + Vec3::Y * GROUND_OFFSET;
    let end = origin + forward * range;
    let side = Vec3::new(-forward.z, 0.0, forward.x) * FORWARD_LANE_HALF_WIDTH;

    gizmos.line(origin, end, color);
    gizmos.line(origin + side, end + side, color);
    gizmos.line(origin - side, end - side, color);
    gizmos.line(end - side, end + side, color);
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevymmo_gameplay::abilities::AbilitySlot;


    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<AbilityAim>();
        app.insert_resource(GameSettingsResource(
            bevymmo_client::user_settings::GameSettings::default(),
        ));
        app.add_systems(Update, cancel_ability_aim_on_escape);
        app
    }

    #[test]
    fn escape_cancels_an_open_aim_and_is_consumed() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<AbilityAim>()
            .begin(AbilitySlot::Primary);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        let aim = *app.world().resource::<AbilityAim>();
        assert!(aim.cancelled, "la mira deve risultare annullata");
        assert_eq!(
            aim.slot,
            Some(AbilitySlot::Primary),
            "lo slot resta occupato finché il tasto non viene rilasciato"
        );
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::Escape),
            "la pressione va consumata, altrimenti si apre anche la pausa"
        );
    }

    #[test]
    fn escape_without_an_aim_is_left_to_pause_and_clear_target() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        assert!(app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::Escape));
    }


}
