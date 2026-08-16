use bevy::prelude::*;
use bevymmo_shared::server_feed::ServerNotice;

use crate::ui::theme::UiTheme;

/// How long a line stays fully visible before it starts to fade.
const HOLD_SECONDS: f32 = 4.0;

/// How long the fade itself takes.
const FADE_SECONDS: f32 = 1.0;

/// Beyond this many lines the oldest is dropped immediately, so a burst of
/// refusals — a key held down against a cooldown — cannot fill the screen.
const MAX_VISIBLE: usize = 6;

/// Where the lines are parented. One node, spawned once.
#[derive(Resource, Default)]
pub struct NoticeLog {
    root: Option<Entity>,
}

/// One line, with the time it has left.
#[derive(Component)]
pub struct Notice {
    age_seconds: f32,
}

pub fn setup_notice_log(mut commands: Commands, mut log: ResMut<NoticeLog>) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(96.0),
                width: Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            // The log must never eat a click meant for the world underneath it.
            Pickable::IGNORE,
        ))
        .id();
    log.root = Some(root);
}

pub fn collect_notices(
    mut commands: Commands,
    mut incoming: MessageReader<ServerNotice>,
    log: Res<NoticeLog>,
    theme: Res<UiTheme>,
    existing: Query<(Entity, &Notice)>,
) {
    let Some(root) = log.root else {
        return;
    };

    // Oldest first, and *owned*: `existing` is a snapshot from the start of the
    // frame, and the despawns below are queued commands. Re-reading the query
    // per notice would keep handing back lines already marked for removal and
    // would never see the ones spawned a moment ago, so a burst arriving in one
    // frame would sail straight past the cap.
    let mut live: Vec<Entity> = {
        let mut by_age: Vec<(Entity, f32)> = existing
            .iter()
            .map(|(entity, notice)| (entity, notice.age_seconds))
            .collect();
        by_age.sort_by(|(_, a), (_, b)| b.total_cmp(a));
        by_age.into_iter().map(|(entity, _)| entity).collect()
    };

    for notice in incoming.read() {
        // The player asked for this and it did not happen, so it also goes to
        // the log file — an on-screen line scrolls away, a log line does not.
        if notice.is_error() {
            warn!("server refused: {}", notice.text);
        } else {
            info!("server: {}", notice.text);
        }

        while live.len() >= MAX_VISIBLE {
            let oldest = live.remove(0);
            commands.entity(oldest).despawn();
        }

        let color = if notice.is_error() {
            theme.error_color
        } else {
            theme.text_color
        };
        let line = commands
            .spawn((
                Notice { age_seconds: 0.0 },
                Text::new(notice.text.clone()),
                TextFont {
                    font_size: FontSize::Px(theme.input_font_size),
                    ..default()
                },
                TextColor(color),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(root).add_child(line);
        live.push(line);
    }
}

pub fn expire_notices(
    mut commands: Commands,
    time: Res<Time>,
    mut notices: Query<(Entity, &mut Notice, &mut TextColor)>,
) {
    let delta = time.delta_secs();
    for (entity, mut notice, mut color) in notices.iter_mut() {
        notice.age_seconds += delta;

        let fading = notice.age_seconds - HOLD_SECONDS;
        if fading <= 0.0 {
            continue;
        }
        if fading >= FADE_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }
        color.0.set_alpha(1.0 - fading / FADE_SECONDS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<UiTheme>();
        app.init_resource::<NoticeLog>();
        app.add_message::<ServerNotice>();
        app.add_systems(Startup, setup_notice_log);
        app.add_systems(Update, (collect_notices, expire_notices).chain());
        app
    }

    #[test]
    fn a_refusal_becomes_one_line() {
        let mut app = app();
        app.update();

        app.world_mut()
            .write_message(ServerNotice::error("could not equip: inventory is full"));
        app.update();

        let mut lines = app.world_mut().query::<&Notice>();
        assert_eq!(lines.iter(app.world()).count(), 1);
    }

    #[test]
    fn a_burst_of_refusals_cannot_fill_the_screen() {
        let mut app = app();
        app.update();

        for _ in 0..(MAX_VISIBLE + 4) {
            app.world_mut()
                .write_message(ServerNotice::error("could not cast: on cooldown"));
        }
        app.update();

        let mut lines = app.world_mut().query::<&Notice>();
        assert_eq!(lines.iter(app.world()).count(), MAX_VISIBLE);
    }
}
