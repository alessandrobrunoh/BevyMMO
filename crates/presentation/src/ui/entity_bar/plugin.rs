//! UI flottante che visualizza nome e punti vita di un'entità.

use super::components::{EntityBarParts, FloatingUi, HpBarFill, HpBarText, NameText};
use super::systems;

use bevy::prelude::*;

use crate::game_state::{not_in_gameplay, Screen};
use crate::renderer::RenderSync;
use crate::ui::{bar::spawn_bar, text::spawn_text, theme::UiTheme};

/// Larghezza della barra HP (px). Usata sia dallo spawn sia dal centraggio
/// orizzontale in `update_floating_ui_position`.
pub(crate) const BAR_WIDTH: f32 = 100.0;

/// Altezza della barra HP (px).
pub(crate) const BAR_HEIGHT: f32 = 14.0;

/// Altezza stimata della riga di testo del nome (px). Approssimazione di
/// `theme.name_font_size` (16.0) usata solo per ancorare il fondo dello stack
/// sopra il punto proiettato.
const NAME_LINE_HEIGHT: f32 = 16.0;

/// Gap verticale tra nome e barra (px). Deve combaciare con `row_gap` dello
/// spawn per mantenere la coerenza del centraggio.
const ROW_GAP: f32 = 4.0;

/// Scostamento della barra sopra un'entità alta come un personaggio.
pub(crate) const CHARACTER_BAR_OFFSET: Vec3 = Vec3::new(0.0, 2.0, 0.0);

/// Aria fra la cima del modello di un nodo raccoglibile e la sua barra: senza,
/// la barra poggia esattamente sulle foglie più alte.
pub(crate) const HARVESTABLE_BAR_CLEARANCE: f32 = 0.6;

/// Altezza totale dello stack (nome + gap + barra) usata da
/// `update_floating_ui_position` per ancorare il fondo dello stack al punto
/// proiettato.
pub(crate) const STACK_HEIGHT: f32 = NAME_LINE_HEIGHT + ROW_GAP + BAR_HEIGHT;

/// Lo stack deve contenere la barra: se qualcuno azzerasse `NAME_LINE_HEIGHT` e
/// `ROW_GAP`, l'ancoraggio in `update_floating_ui_position` taglierebbe la barra.
/// Verificato a compile time, non in un `#[test]`: sono costanti, quindi un
/// assert a runtime non aggiunge nulla e clippy lo segnala giustamente.
const _: () = assert!(STACK_HEIGHT > BAR_HEIGHT);

pub struct EntityBarPlugin;

impl Plugin for EntityBarPlugin {
    fn build(&self, app: &mut App) {
        // La barra di un nodo raccoglibile legge il massimo dei pezzi dal
        // registry. `PresentationCorePlugin` lo popola allo Startup; qui lo si
        // dichiara comunque perché i sistemi lo richiedono, e `init_resource`
        // non sovrascrive quello già presente.
        app.init_resource::<bevymmo_gameplay::placeables::PlaceableRegistry>();
        // Nota: nessun observer su `Add<VitalStats>`. Osservatore e
        // `spawn_ui_for_new_entities` facevano lo stesso lavoro; entrambi
        // chiamavano `get_or_spawn_root`, la cui `Query` non vede un root
        // appena spawnato e non ancora flushato, quindi nello stesso frame si
        // potevano creare due root e due barre per la stessa entità. Il
        // sistema di polling è idempotente (guardia `FloatingUiAttached`),
        // riusa il root all'interno della stessa esecuzione ed è già gatato su
        // `in_gameplay` — l'observer non lo era.
        app.add_systems(
            Update,
            (
                systems::spawn_ui_for_new_entities,
                (
                    // In `RenderSync::Project`: the projection needs both the
                    // target's smoothed transform and the camera's, as written
                    // this frame. Reading either a frame late makes the bar
                    // drift against the character while walking.
                    systems::update_floating_ui_position.in_set(RenderSync::Project),
                    systems::update_floating_ui_content,
                    systems::cleanup_floating_ui,
                )
                    .chain(),
            )
                .chain()
                .run_if(in_state(Screen::InGame)),
        )
        // Fuori dal gameplay la UI flottante viene smontata (root + figli) e il
        // marker `FloatingUiAttached` resettato, così un nuovo match parte
        // pulito.
        .add_systems(
            Update,
            systems::cleanup_floating_ui_root.run_if(not_in_gameplay),
        );
    }
}

/// Genera una barra UI flottante legata a `target`, agganciandola a `parent_ui_node`.
///
/// `world_offset` è lo scostamento dal punto ancorato al quale lo stack viene
/// proiettato: un personaggio lo vuole sopra la testa, un albero sopra la chioma.
///
/// Inserisce [`EntityBarParts`] sul container per consentire aggiornamenti diretti
/// con cache dei valori già applicati.
pub fn spawn_entity_bar(
    commands: &mut Commands,
    parent_ui_node: Entity,
    target: Entity,
    world_offset: Vec3,
    theme: &UiTheme,
) -> Entity {
    let container = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(ROW_GAP),
                // Nascosto finché la fase di posizione non produce una
                // proiezione valida; evita flash in alto a sinistra.
                display: Display::None,
                ..default()
            },
            FloatingUi {
                target,
                offset: world_offset,
                last_viewport: None,
            },
            Pickable::IGNORE,
        ))
        .id();

    commands.entity(parent_ui_node).add_child(container);

    // Backdrop opaco per il nome: chip con padding 2px e colore panel.
    // Migliora drasticamente la leggibilità sopra uno sfondo 3D luminoso.
    let name_backdrop = commands
        .spawn((
            Node {
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(container).add_child(name_backdrop);

    let name_text = spawn_text(
        commands,
        name_backdrop,
        "Loading...",
        theme.name_font_size,
        theme.text_color,
    );
    commands
        .entity(name_text)
        .insert((NameText, Pickable::IGNORE));

    let (bar, fill) = spawn_bar(
        commands,
        container,
        1.0,
        1.0,
        Vec2::new(BAR_WIDTH, BAR_HEIGHT),
        theme.bar_bg,
        theme.hp_fill,
    );
    commands.entity(bar).insert(Pickable::IGNORE);
    commands.entity(fill).insert((HpBarFill, Pickable::IGNORE));

    let hp_text = spawn_text(commands, bar, "?/?", theme.hp_font_size, theme.text_color);
    commands
        .entity(hp_text)
        .insert((HpBarText, Pickable::IGNORE));

    commands
        .entity(container)
        .insert(EntityBarParts::new(name_text, fill, hp_text));

    container
}
