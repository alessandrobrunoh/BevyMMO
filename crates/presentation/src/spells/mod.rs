//! Client presentation for spells: cast bars, HUD and visual effects.

pub mod available_choices;
pub mod cast_bar;
pub mod dragon_enemy;
pub mod effects;
pub mod eidolon_effects;
pub mod eidolon_input;
pub mod healing_circle;
pub mod input;
pub mod meteorite;
pub mod ray_of_light;
pub mod stun_field;
pub mod ui;

use bevy::prelude::*;
use bevymmo_shared::network::protocol::SpellVisualEffect;

/// Registers spell HUD, cast-bar and client visual systems.
pub struct SpellsHudPlugin;

impl Plugin for SpellsHudPlugin {
    fn build(&self, app: &mut App) {
        ui::spell_hud_systems(app);
        cast_bar::cast_bar_systems(app);
        app.add_systems(
            Update,
            (
                available_choices::sync_available_spell_choices,
                input::cast_spells_on_key,
                eidolon_input::cast_eidolon_abilities_on_key,
                dispatch_visual_effects,
                eidolon_effects::animate,
                healing_circle::visual::animate,
                meteorite::visual::animate,
                ray_of_light::visual::animate,
                stun_field::visual::animate,
                dragon_enemy::cataclysm::visual::animate,
                dragon_enemy::dragon_claw::visual::animate,
                dragon_enemy::molten_eruption::visual::animate,
                dragon_enemy::searing_breath::visual::animate,
                dragon_enemy::tail_sweep::visual::animate,
                dragon_enemy::wing_buffet::visual::animate,
            )
                .run_if(bevymmo_shared::network::mode::has_client),
        );
    }
}

fn dispatch_visual_effects(
    mut commands: Commands,
    mut effects: MessageReader<SpellVisualEffect>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    abilities: Res<bevymmo_shared::abilities::BaseAbilityRegistry>,
) {
    for effect in effects.read() {
        match effect.spell_id.as_str() {
            "healing_circle" => {
                healing_circle::visual::spawn(&mut commands, &mut meshes, &mut materials, effect)
            }
            "meteorite" => {
                meteorite::visual::spawn(&mut commands, &mut meshes, &mut materials, effect)
            }
            "ray_of_light" => {
                ray_of_light::visual::spawn(&mut commands, &mut meshes, &mut materials, effect)
            }
            "stun_field" => {
                stun_field::visual::spawn(&mut commands, &mut meshes, &mut materials, effect)
            }
            "cataclysm" => dragon_enemy::cataclysm::visual::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                effect,
            ),
            "dragon_claw" => dragon_enemy::dragon_claw::visual::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                effect,
            ),
            "molten_eruption" => dragon_enemy::molten_eruption::visual::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                effect,
            ),
            "searing_breath" => dragon_enemy::searing_breath::visual::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                effect,
            ),
            "tail_sweep" => dragon_enemy::tail_sweep::visual::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                effect,
            ),
            "wing_buffet" => dragon_enemy::wing_buffet::visual::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                effect,
            ),
            // Un gesto Eidolon manda il proprio id: il visual si costruisce
            // rileggendo la sua `BaseAbility` (forma, raggio, preavviso), così
            // un gesto nuovo si vede senza scrivere un visual dedicato.
            other => {
                let ability = abilities.get(&bevymmo_shared::abilities::AbilityId::new(other.to_string()));
                match ability {
                    Some(ability) => eidolon_effects::spawn_for_ability(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        effect,
                        &ability,
                    ),
                    None => eidolon_effects::spawn(&mut commands, &mut meshes, &mut materials, effect),
                }
            }
        }
    }
}
