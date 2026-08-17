//! [`AbilityAim`] — la finestra di mira aperta fra la pressione e il rilascio
//! del tasto di un'abilità Eidolon.
//!
//! Vive qui e non nel crate di presentazione per la stessa ragione di
//! [`crate::targeting::CurrentTarget`]: la leggono sia `bevymmo_presentation`
//! (che disegna l'anteprima e invia il cast) sia `bevymmo_client` (che deve
//! sapere se Esc è già stato consumato per annullare la mira invece di
//! deselezionare il bersaglio).

use super::slot::AbilitySlot;
use glam::Vec3;

/// Stato della mira in corso. Puramente client-side: il server non la vede
/// mai, riceve solo il `EidolonCastCommand` finale.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct AbilityAim {
    /// Slot il cui tasto è attualmente premuto, se si sta mirando.
    pub slot: Option<AbilitySlot>,
    /// Mira annullata con Esc: al rilascio non parte nessun cast. Resta
    /// `true` finché il tasto non viene rilasciato, altrimenti il gesto
    /// ripartirebbe da solo continuando a tenere premuto.
    pub cancelled: bool,
    /// Punto di terra sotto il cursore, aggiornato ogni frame mentre si mira.
    pub ground_point: Option<Vec3>,
}

impl AbilityAim {
    /// `true` se si sta mirando e la mira non è stata annullata — cioè se
    /// c'è qualcosa da disegnare a terra.
    pub fn is_active(&self) -> bool {
        self.slot.is_some() && !self.cancelled
    }

    /// Apre la mira su `slot`, scartando qualunque mira precedente.
    pub fn begin(&mut self, slot: AbilitySlot) {
        self.slot = Some(slot);
        self.cancelled = false;
    }

    /// Chiude la mira e torna allo stato di riposo.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_aiming() {
        assert!(!AbilityAim::default().is_active());
    }

    #[test]
    fn begin_opens_the_aim_and_resets_a_previous_cancellation() {
        let mut aim = AbilityAim {
            cancelled: true,
            ..Default::default()
        };
        aim.begin(AbilitySlot::Primary);
        assert_eq!(aim.slot, Some(AbilitySlot::Primary));
        assert!(aim.is_active());
    }

    #[test]
    fn a_cancelled_aim_is_not_active_but_stays_held() {
        let mut aim = AbilityAim::default();
        aim.begin(AbilitySlot::Secondary);
        aim.cancelled = true;
        // Il tasto è ancora premuto: lo slot resta, così il rilascio sa che
        // deve scartare invece di lanciare.
        assert_eq!(aim.slot, Some(AbilitySlot::Secondary));
        assert!(!aim.is_active());
    }

    #[test]
    fn clear_returns_to_rest() {
        let mut aim = AbilityAim::default();
        aim.begin(AbilitySlot::Ultimate);
        aim.ground_point = Some(Vec3::X);
        aim.clear();
        assert_eq!(aim, AbilityAim::default());
    }
}
