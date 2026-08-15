//! Riepilogo testuale di un'arma per la scheda item.
//!
//! Solo funzioni pure: prendono catalogo + esemplare + registri e ritornano
//! stringhe già formattate, così la parte di `detail.rs` che costruisce i
//! `Node` resta banale e questo file è testabile senza un `App`.
//!
//! I numeri mostrati passano da [`resolve_slot_preview`], cioè esattamente la
//! stessa risoluzione che il cast usa un istante dopo: leggere `base_params()`
//! qui mostrerebbe valori che il gioco poi non applica non appena c'è un
//! Modificatore inciso.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use bevymmo_shared::abilities::{
    inscription::inscription_cost, resolve_active_ability, resolve_slot_preview, AbilitySlot,
    AbilityTag, AncientWordRegistry, BaseAbilityRegistry, CastBlockedReason, EssenceRegistry,
    Inscription, KnownGlyphs, ModifierRegistry, RuneProfile, SlotPreview, WeaponInscriptions,
};
use bevymmo_shared::items::{instance::ItemInstance, ItemCategory, ItemRarity, ItemRegistry};

/// I registri necessari a descrivere un'arma incisa, raggruppati per non far
/// crescere la firma della scheda item di un argomento per tipo di Glifo.
#[derive(SystemParam)]
pub struct GlyphRegistries<'w> {
    pub abilities: Res<'w, BaseAbilityRegistry>,
    pub essences: Res<'w, EssenceRegistry>,
    pub modifiers: Res<'w, ModifierRegistry>,
    pub ancient_words: Res<'w, AncientWordRegistry>,
}

impl GlyphRegistries<'_> {
    pub fn catalog(&self) -> GlyphCatalog<'_> {
        GlyphCatalog {
            abilities: &self.abilities,
            essences: &self.essences,
            modifiers: &self.modifiers,
            ancient_words: &self.ancient_words,
        }
    }
}

/// Gli stessi quattro registri per riferimento semplice.
///
/// Le funzioni di questo modulo prendono questo, non [`GlyphRegistries`]:
/// restano così pure e collaudabili senza costruire un `App` solo per
/// soddisfare un `SystemParam`.
#[derive(Clone, Copy)]
pub struct GlyphCatalog<'a> {
    pub abilities: &'a BaseAbilityRegistry,
    pub essences: &'a EssenceRegistry,
    pub modifiers: &'a ModifierRegistry,
    pub ancient_words: &'a AncientWordRegistry,
}

/// Riga "Rune" della scheda.
#[derive(Debug, Clone, PartialEq)]
pub struct RuneSummary {
    pub used: u32,
    pub capacity: u32,
    pub stability: f32,
    pub affinity: Option<String>,
}

impl RuneSummary {
    pub fn line(&self) -> String {
        let mut parts = vec![
            format!("{}/{} capacity", self.used, self.capacity),
            format!("{:.0}% stability", self.stability * 100.0),
        ];
        if let Some(affinity) = &self.affinity {
            parts.push(format!("{affinity} affinity"));
        }
        parts.join("  ·  ")
    }
}

/// Un blocco "slot" della scheda: il gesto attivo e tutto ciò che lo descrive.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotSummary {
    /// `Primary` / `Secondary` / `Ultimate` — non `Q`/`W`/`E`: il tasto è
    /// rebindabile e non è un dato dell'arma (vedi `AbilitySlot`).
    pub slot: &'static str,
    pub title: String,
    /// `Some` quando lo slot è inutilizzabile: un Glifo inciso non è nel
    /// Vocabolario del personaggio, quindi il cast verrebbe rifiutato in
    /// blocco (§ "blocco totale" di `cast_inscribed_slot`).
    pub blocked: Option<String>,
    pub shape: String,
    pub stats: String,
    pub tags: String,
    pub glyphs: Option<String>,
    /// Le altre opzioni offerte dall'arma per questo slot, se più di una.
    pub alternatives: Option<String>,
}

/// Tutto ciò che la scheda mostra in più per un'arma Eidolon.
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponSummary {
    pub runes: Option<RuneSummary>,
    pub slots: Vec<SlotSummary>,
}

/// Riga di intestazione comune a QUALUNQUE item (anche non arma).
pub fn meta_line(
    category: ItemCategory,
    rarity: ItemRarity,
    equip_slot: Option<impl std::fmt::Debug>,
    weight: f32,
) -> String {
    let mut parts = vec![format!("{category:?}"), format!("{rarity:?}")];
    if let Some(slot) = equip_slot {
        parts.push(format!("{slot:?}"));
    }
    if weight > 0.0 {
        parts.push(format!("{} wt", number(weight)));
    }
    parts.join("  ·  ")
}

/// Costruisce il riepilogo Eidolon di `instance`, o `None` se l'item non è
/// un'arma con gesti propri (armor, pozioni, armi sul vecchio modello
/// `spell_kit`).
pub fn summarize_weapon(
    instance: &ItemInstance,
    items: &ItemRegistry,
    glyphs: GlyphCatalog,
    known: &KnownGlyphs,
) -> Option<WeaponSummary> {
    let item = items.get(&instance.item_id)?;
    let abilities = item.weapon_abilities()?;
    let inscriptions = instance.inscriptions.clone().unwrap_or_default();
    let profile = item.rune_profile();

    let runes = profile.map(|profile| RuneSummary {
        used: total_rune_cost(&inscriptions, profile, glyphs),
        capacity: profile.capacity,
        stability: profile.stability,
        affinity: profile.affinity.as_ref().and_then(|id| {
            glyphs
                .essences
                .get(id)
                .map(|essence| essence.display_name().to_string())
        }),
    });

    let slots = AbilitySlot::ALL
        .iter()
        .filter_map(|&slot| summarize_slot(slot, instance, abilities, &inscriptions, glyphs, known))
        .collect();

    Some(WeaponSummary { runes, slots })
}

/// Somma il costo dei tre slot: la Capacità Runica è condivisa fra loro, non
/// per-slot (§39-41), quindi la scheda deve mostrare il totale.
fn total_rune_cost(
    inscriptions: &WeaponInscriptions,
    profile: &RuneProfile,
    glyphs: GlyphCatalog,
) -> u32 {
    AbilitySlot::ALL
        .iter()
        .map(|&slot| {
            inscription_cost(
                inscriptions.get(slot),
                profile,
                glyphs.essences,
                glyphs.modifiers,
                glyphs.ancient_words,
            )
        })
        .sum()
}

fn summarize_slot(
    slot: AbilitySlot,
    instance: &ItemInstance,
    abilities: &bevymmo_shared::abilities::WeaponAbilities,
    inscriptions: &WeaponInscriptions,
    glyphs: GlyphCatalog,
    known: &KnownGlyphs,
) -> Option<SlotSummary> {
    let selection = &instance.ability_selection;
    let active_id = resolve_active_ability(slot, abilities, selection)?;
    let ability = glyphs.abilities.get(active_id)?;
    let inscription = inscriptions.get(slot);

    // `resolve_slot_preview` è la prima metà del cast: risolve il gesto e
    // applica i Modificatori. Se fallisce per Glifo sconosciuto la scheda lo
    // dice invece di mostrare numeri che il gioco non userebbe — ma i
    // parametri base restano utili, quindi si continua comunque.
    let preview = resolve_slot_preview(
        slot,
        abilities,
        selection,
        inscriptions,
        known,
        glyphs.abilities,
        glyphs.modifiers,
    );
    let blocked = match &preview {
        Err(CastBlockedReason::UnknownGlyph) => {
            Some("Locked — you don't know every inscribed Glyph".to_string())
        }
        Err(reason) => Some(format!("Unavailable — {reason:?}")),
        Ok(_) => None,
    };
    let params = match preview {
        Ok(SlotPreview { params, .. }) => params,
        Err(_) => bevymmo_shared::abilities::resolve_ability_params(
            ability.base_params(),
            &inscription.modifiers,
            glyphs.modifiers,
        ),
    };

    let essence_name = inscription
        .essence
        .as_ref()
        .and_then(|id| glyphs.essences.get(id))
        .map(|essence| essence.display_name().to_string());
    let title = match &essence_name {
        Some(name) => format!("{} — {name}", ability.display_name()),
        None => format!("{} — physical", ability.display_name()),
    };

    let alternatives = {
        let others: Vec<String> = abilities
            .options_for(slot)
            .iter()
            .filter(|id| *id != active_id)
            .filter_map(|id| glyphs.abilities.get(id))
            .map(|other| other.display_name().to_string())
            .collect();
        (!others.is_empty()).then(|| format!("Also offers: {}", others.join(", ")))
    };

    Some(SlotSummary {
        slot: slot_name(slot),
        title,
        blocked,
        shape: describe_geometry(ability.geometry()),
        stats: describe_params(&params),
        tags: describe_tags(ability.tags()),
        glyphs: describe_glyphs(inscription, glyphs),
        alternatives,
    })
}

const fn slot_name(slot: AbilitySlot) -> &'static str {
    match slot {
        AbilitySlot::Primary => "Primary",
        AbilitySlot::Secondary => "Secondary",
        AbilitySlot::Ultimate => "Ultimate",
    }
}

fn describe_geometry(geometry: bevymmo_shared::abilities::AbilityGeometry) -> String {
    use bevymmo_shared::abilities::AbilityGeometry::*;
    match geometry {
        Cone { radius, angle_deg } => {
            format!("Cone {} m / {}°", number(radius), number(angle_deg))
        }
        Circle { radius } => format!("Circle {} m", number(radius)),
        Projectile { range, speed } => {
            format!("Projectile {} m @ {} m/s", number(range), number(speed))
        }
        SelfBuff { duration_seconds } => format!("Self buff {} s", number(duration_seconds)),
    }
}

/// Solo i campi che portano informazione: un `0` ovunque è il default di
/// `AbilityParams`, e stamparlo riempirebbe la riga di rumore.
fn describe_params(params: &bevymmo_shared::abilities::AbilityParams) -> String {
    let mut parts = Vec::new();
    if params.power != 0.0 {
        parts.push(format!("{} power", number(params.power)));
    }
    if params.area != 0.0 {
        parts.push(format!("{} m area", number(params.area)));
    }
    if params.range != 0.0 {
        parts.push(format!("{} m range", number(params.range)));
    }
    if params.cast_time != 0.0 {
        parts.push(format!("{} s cast", number(params.cast_time)));
    }
    if params.cooldown != 0.0 {
        parts.push(format!("{} s cooldown", number(params.cooldown)));
    }
    if params.energy_cost != 0.0 {
        parts.push(format!("{} mana", number(params.energy_cost)));
    }
    if parts.is_empty() {
        return "—".to_string();
    }
    parts.join("  ·  ")
}

fn describe_tags(tags: &[AbilityTag]) -> String {
    if tags.is_empty() {
        return "—".to_string();
    }
    tags.iter()
        .map(|tag| format!("{tag:?}"))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// L'incisione dello slot, con il costo runico di ogni Glifo: è il numero che
/// spiega perché la Capacità è quella che è.
fn describe_glyphs(inscription: &Inscription, glyphs: GlyphCatalog) -> Option<String> {
    if inscription.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    if let Some(essence) = inscription
        .essence
        .as_ref()
        .and_then(|id| glyphs.essences.get(id))
    {
        parts.push(format!(
            "{} ({})",
            essence.display_name(),
            essence.rune_cost()
        ));
    }
    for modifier in inscription
        .modifiers
        .iter()
        .filter_map(|id| glyphs.modifiers.get(id))
    {
        parts.push(format!(
            "{} ({})",
            modifier.display_name(),
            modifier.rune_cost()
        ));
    }
    if let Some(word) = inscription
        .ancient_word
        .as_ref()
        .and_then(|id| glyphs.ancient_words.get(id))
    {
        parts.push(format!("{} ({})", word.display_name(), word.rune_cost()));
    }

    (!parts.is_empty()).then(|| parts.join(" + "))
}

/// Formato compatto: `2.5` resta `2.5`, `22.0` diventa `22`. Una scheda piena
/// di `.0` inutili è più difficile da leggere a colpo d'occhio.
fn number(value: f32) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{:.0}", value.round())
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_shared::abilities::{AbilityGeometry, AbilityParams};
    use bevymmo_shared::items::components::EquipSlot;

    fn params() -> AbilityParams {
        AbilityParams {
            power: 220.0,
            area: 0.0,
            range: 22.0,
            cast_time: 0.25,
            cooldown: 2.5,
            energy_cost: 10.0,
        }
    }

    /// Builds the four registries with the game's real content, so the
    /// summary is exercised against the same data the player sees.
    fn catalog_app() -> App {
        let mut app = App::new();
        app.init_resource::<ItemRegistry>()
            .init_resource::<BaseAbilityRegistry>()
            .init_resource::<EssenceRegistry>()
            .init_resource::<ModifierRegistry>()
            .init_resource::<AncientWordRegistry>();
        app.add_systems(
            Startup,
            (
                bevymmo_shared::items_impl::register_default_items,
                bevymmo_shared::base_abilities_impl::register_default_base_abilities,
                bevymmo_shared::essences_impl::register_default_essences,
                bevymmo_shared::modifiers_impl::register_default_modifiers,
                bevymmo_shared::ancient_words_impl::register_default_ancient_words,
            ),
        );
        app.update();
        app
    }

    fn summarize(app: &App, instance: &ItemInstance, known: &KnownGlyphs) -> WeaponSummary {
        let items = app.world().resource::<ItemRegistry>();
        let catalog = GlyphCatalog {
            abilities: app.world().resource::<BaseAbilityRegistry>(),
            essences: app.world().resource::<EssenceRegistry>(),
            modifiers: app.world().resource::<ModifierRegistry>(),
            ancient_words: app.world().resource::<AncientWordRegistry>(),
        };
        summarize_weapon(instance, items, catalog, known).expect("magic_staff is an Eidolon weapon")
    }

    fn magic_staff() -> ItemInstance {
        ItemInstance::new(bevymmo_shared::items::ItemId::new("magic_staff"))
    }

    /// A weapon with no inscription still describes all three of its gestures.
    #[test]
    fn a_virgin_weapon_summarizes_every_slot() {
        let app = catalog_app();
        let summary = summarize(&app, &magic_staff(), &KnownGlyphs::default());

        assert_eq!(summary.slots.len(), 3);
        assert_eq!(summary.slots[0].slot, "Primary");
        assert_eq!(summary.slots[2].slot, "Ultimate");
        assert!(summary.slots.iter().all(|slot| slot.blocked.is_none()));
        assert!(summary.slots.iter().all(|slot| slot.glyphs.is_none()));

        let runes = summary.runes.expect("magic_staff has a rune profile");
        assert_eq!(runes.used, 0);
        assert_eq!(runes.capacity, 8);
        assert_eq!(runes.affinity.as_deref(), Some("Fuoco"));
    }

    /// Primary offers two gestures, so the card must name the one that is not
    /// active — that is how the player learns the weapon can be re-selected.
    #[test]
    fn slots_with_a_choice_list_the_other_option() {
        let app = catalog_app();
        let summary = summarize(&app, &magic_staff(), &KnownGlyphs::default());

        let primary = &summary.slots[0];
        let alternatives = primary
            .alternatives
            .as_ref()
            .expect("Primary offers two gestures");
        assert!(
            alternatives.starts_with("Also offers: "),
            "got: {alternatives}"
        );
        assert!(!alternatives.contains(&primary.title));

        // Ultimate has exactly one gesture: nothing to offer.
        assert!(summary.slots[2].alternatives.is_none());
    }

    /// The rune line is the shared budget across all three slots, discounted by
    /// the weapon's Affinity — Fuoco costs 2 but 1 on a Fuoco-affine staff.
    #[test]
    fn inscribed_glyphs_are_listed_with_their_rune_cost() {
        let app = catalog_app();
        let mut instance = magic_staff();
        instance.inscriptions = Some(WeaponInscriptions {
            primary: Inscription {
                essence: Some(bevymmo_shared::abilities::EssenceId::new("fuoco")),
                modifiers: vec![bevymmo_shared::abilities::ModifierId::new("amplificare")],
                ancient_word: None,
            },
            ..Default::default()
        });

        let mut known = KnownGlyphs::default();
        known
            .essences
            .insert(bevymmo_shared::abilities::EssenceId::new("fuoco"));
        known
            .modifiers
            .insert(bevymmo_shared::abilities::ModifierId::new("amplificare"));

        let summary = summarize(&app, &instance, &known);
        let primary = &summary.slots[0];

        assert!(primary.blocked.is_none(), "every glyph is known");
        assert!(primary.title.contains("Fuoco"), "got: {}", primary.title);
        let glyphs = primary.glyphs.as_ref().expect("primary is inscribed");
        assert!(glyphs.contains("Fuoco (2)"), "got: {glyphs}");
        assert!(glyphs.contains("Amplificare (3)"), "got: {glyphs}");

        // Fuoco (2) discounted to 1 by the staff's Fuoco affinity, + 3.
        let runes = summary.runes.expect("rune profile");
        assert_eq!(runes.used, 4);
    }

    /// The regression this guards: a weapon found already inscribed by someone
    /// else is unusable on that slot until every Glyph is learned. The card has
    /// to say so rather than showing numbers the cast will never apply.
    #[test]
    fn a_slot_with_an_unknown_glyph_is_marked_locked() {
        let app = catalog_app();
        let mut instance = magic_staff();
        instance.inscriptions = Some(WeaponInscriptions {
            primary: Inscription {
                essence: Some(bevymmo_shared::abilities::EssenceId::new("fuoco")),
                modifiers: vec![],
                ancient_word: None,
            },
            ..Default::default()
        });

        // Empty Vocabulary: the player knows nothing.
        let summary = summarize(&app, &instance, &KnownGlyphs::default());

        assert!(summary.slots[0].blocked.is_some());
        assert!(
            summary.slots[1].blocked.is_none(),
            "an uninscribed slot stays usable"
        );
        // Still described: a locked slot must not become a blank block.
        assert!(!summary.slots[0].stats.is_empty());
    }

    #[test]
    fn number_drops_a_meaningless_decimal_but_keeps_a_real_one() {
        assert_eq!(number(22.0), "22");
        assert_eq!(number(2.5), "2.5");
        assert_eq!(number(0.25), "0.2");
    }

    #[test]
    fn params_line_lists_only_the_fields_that_carry_information() {
        let line = describe_params(&params());
        assert!(line.contains("220 power"));
        assert!(line.contains("22 m range"));
        assert!(line.contains("2.5 s cooldown"));
        // `area` is 0 for a pure projectile: printing it would be noise.
        assert!(!line.contains("area"), "got: {line}");
    }

    #[test]
    fn params_line_never_comes_back_empty() {
        let empty = AbilityParams {
            power: 0.0,
            area: 0.0,
            range: 0.0,
            cast_time: 0.0,
            cooldown: 0.0,
            energy_cost: 0.0,
        };
        assert_eq!(describe_params(&empty), "—");
    }

    #[test]
    fn geometry_is_described_per_shape() {
        assert_eq!(
            describe_geometry(AbilityGeometry::Projectile {
                range: 22.0,
                speed: 24.0
            }),
            "Projectile 22 m @ 24 m/s"
        );
        assert_eq!(
            describe_geometry(AbilityGeometry::Circle { radius: 4.5 }),
            "Circle 4.5 m"
        );
        assert_eq!(
            describe_geometry(AbilityGeometry::Cone {
                radius: 8.0,
                angle_deg: 60.0
            }),
            "Cone 8 m / 60°"
        );
    }

    #[test]
    fn tags_fall_back_to_a_dash_when_empty() {
        assert_eq!(describe_tags(&[]), "—");
        assert_eq!(
            describe_tags(&[AbilityTag::Ranged, AbilityTag::Projectile]),
            "Ranged · Projectile"
        );
    }

    #[test]
    fn rune_line_reports_usage_against_capacity() {
        let runes = RuneSummary {
            used: 6,
            capacity: 8,
            stability: 0.96,
            affinity: Some("Fuoco".to_string()),
        };
        let line = runes.line();
        assert!(line.contains("6/8 capacity"), "got: {line}");
        assert!(line.contains("96% stability"), "got: {line}");
        assert!(line.contains("Fuoco affinity"), "got: {line}");
    }

    #[test]
    fn rune_line_omits_affinity_when_the_weapon_has_none() {
        let runes = RuneSummary {
            used: 0,
            capacity: 4,
            stability: 1.0,
            affinity: None,
        };
        assert!(!runes.line().contains("affinity"));
    }

    #[test]
    fn meta_line_skips_a_weightless_inventory_only_item() {
        let line = meta_line(
            ItemCategory::Material,
            ItemRarity::Common,
            None::<EquipSlot>,
            0.0,
        );
        assert_eq!(line, "Material  ·  Common");
    }

    #[test]
    fn meta_line_includes_the_equip_slot_when_there_is_one() {
        let line = meta_line(
            ItemCategory::Weapon,
            ItemRarity::Rare,
            Some(EquipSlot::Weapon),
            1.5,
        );
        assert!(line.contains("Weapon"));
        assert!(line.contains("Rare"));
        assert!(line.contains("1.5 wt"));
    }
}
