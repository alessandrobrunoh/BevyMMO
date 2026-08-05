//! Visualizzazione temporanea dell'area di attacco dell'Enemy.
//!
//! Questo modulo è volutamente separato dalla logica di combat: può essere
//! rimosso insieme alla sua registrazione nel plugin senza modificare il danno.

use bevy::prelude::*;

use super::components::EnemyAttack;
use crate::game_state::{GameScreen, Screen};
use crate::network::protocol::Position;
use crate::network::mode::has_client;

#[derive(Component)]
struct EnemyAttackIndicator;

pub fn spawn_attack_indicators(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    enemies: Query<(Entity, &EnemyAttack, Option<&Children>), With<Position>>,
    indicators: Query<(), With<EnemyAttackIndicator>>,
) {
    for (enemy, attack, children) in enemies.iter() {
        let already_spawned = children
            .is_some_and(|children| children.iter().any(|child| indicators.get(child).is_ok()));
        if already_spawned {
            continue;
        }

        let mesh = meshes.add(Cylinder::new(attack.radius.max(0.0), 0.04));
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.0, 0.0, 0.3),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });

        commands.entity(enemy).with_children(|parent| {
            parent.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(0.0, 0.03, 0.0),
                EnemyAttackIndicator,
            ));
        });
    }
}

pub fn cleanup_attack_indicators(
    mut commands: Commands,
    indicators: Query<Entity, With<EnemyAttackIndicator>>,
) {
    for indicator in indicators.iter() {
        commands.entity(indicator).despawn();
    }
}

pub fn in_gameplay(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

pub fn not_in_gameplay(screen: Res<GameScreen>) -> bool {
    !in_gameplay(screen)
}

pub fn client_debug_systems(app: &mut App) {
    app.add_systems(
        Update,
        spawn_attack_indicators
            .run_if(has_client)
            .run_if(in_gameplay),
    )
    .add_systems(
        Update,
        cleanup_attack_indicators
            .run_if(has_client)
            .run_if(not_in_gameplay),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameScreen;
    use crate::network::protocol::Position;

    #[test]
    fn indicator_is_spawned_once_for_an_enemy_attack_area() {
        let mut app = App::new();
        app.init_resource::<GameScreen>();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.add_systems(Update, spawn_attack_indicators);

        let enemy = app
            .world_mut()
            .spawn((Position(Vec3::ZERO), EnemyAttack::default()))
            .id();
        app.update();
        app.update();

        let children = app.world().entity(enemy).get::<Children>().expect("indicator child");
        assert_eq!(children.len(), 1);
        let indicator = children[0];
        assert!(app.world().entity(indicator).contains::<EnemyAttackIndicator>());
    }
}
