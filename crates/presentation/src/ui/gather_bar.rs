//! Local gather progress bar.

use bevy::prelude::*;

use crate::game_state::in_gameplay;
use crate::ui::theme::UiTheme;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_gameplay::gathering::ActiveGather;

pub struct GatherBarPlugin;

impl Plugin for GatherBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_gather_bar);
        app.add_systems(Update, update_gather_bar.run_if(in_gameplay));
    }
}

#[derive(Component)]
struct GatherBarRoot;

#[derive(Component)]
struct GatherBarFill;

#[derive(Component)]
struct GatherBarLabel;

fn setup_gather_bar(mut commands: Commands, theme: Res<UiTheme>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(120.0),
                left: Val::Percent(50.0),
                width: Val::Px(240.0),
                height: Val::Px(22.0),
                margin: UiRect::left(Val::Px(-120.0)),
                border: UiRect::all(Val::Px(2.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.8)),
            BorderColor::all(theme.input_border),
            GatherBarRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.85, 0.55, 0.15)),
                GatherBarFill,
            ));
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(theme.text_color),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                GatherBarLabel,
            ));
        });
}

fn update_gather_bar(
    gather: Query<&ActiveGather, With<LocalPlayer>>,
    mut root: Query<&mut Node, With<GatherBarRoot>>,
    mut fill: Query<&mut Node, (With<GatherBarFill>, Without<GatherBarRoot>)>,
    mut label: Query<&mut Text, With<GatherBarLabel>>,
) {
    let gathering = gather.iter().next();
    let Ok(mut root) = root.single_mut() else {
        return;
    };
    match gathering {
        None => {
            root.display = Display::None;
        }
        Some(gather) => {
            root.display = Display::Flex;
            let pct = if gather.required_seconds <= 0.0 {
                1.0
            } else {
                (gather.elapsed_seconds / gather.required_seconds).clamp(0.0, 1.0)
            };
            if let Ok(mut fill) = fill.single_mut() {
                fill.width = Val::Percent(pct * 100.0);
            }
            if let Ok(mut text) = label.single_mut() {
                let remaining = (gather.required_seconds - gather.elapsed_seconds).max(0.0);
                text.0 = format!("Gathering {remaining:.1}s");
            }
        }
    }
}
