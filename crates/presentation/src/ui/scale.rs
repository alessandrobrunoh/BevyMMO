//! Conversions between the three pixel spaces Bevy's UI juggles.
//!
//! Getting these wrong is silent: everything lines up at `UiScale == 1.0` on a
//! non-HiDPI screen, and drifts proportionally as soon as either factor differs
//! from unity. That has already caused three separate bugs here — the item drag
//! ghost, the draggable card, and the scrollbar travel ratio — so the
//! conversions live in one place with their reasoning attached.
//!
//! The three spaces:
//!
//! | Space | Where you get it | Relation to physical |
//! |---|---|---|
//! | **physical** | `ComputedNode::size()`, `UiGlobalTransform::translation` | — |
//! | **window-logical** | `Window::cursor_position()`, `Camera::world_to_viewport` | `physical / window.scale_factor()` |
//! | **UI-logical** (`Val::Px`) | what you write into `Node::left`/`top` | `physical / (window.scale_factor() * UiScale)` |
//!
//! Bevy computes UI layout at `target_scaling_factor * ui_scale`
//! (`bevy_ui::update`), which is why `Val::Px` and `cursor_position()` disagree
//! by exactly `UiScale` — the factor that is easy to forget because it is 1.0
//! until a player touches the interface-scale setting.

use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiScale};

/// Converts a window-logical position — a cursor position, or anything out of
/// [`Camera::world_to_viewport`] — into the space [`Val::Px`] is interpreted in.
///
/// Use this whenever a UI node has to be placed *at* something the pointer or
/// the 3D camera reported. Both sides already divide out the window's DPI scale,
/// so `UiScale` is the only factor left to remove.
#[inline]
pub fn window_to_ui_px(window_logical: Vec2, ui_scale: &UiScale) -> Vec2 {
    window_logical / ui_scale.0
}

/// Converts a physical-pixel UI coordinate — anything read back out of
/// [`ComputedNode`] or [`UiGlobalTransform`] — into the space [`Val::Px`] is
/// interpreted in.
///
/// Needed when a system reads a node's laid-out geometry and writes it back as
/// an explicit anchor: the read is physical, the write is UI-logical, and
/// round-tripping without this scales the node's position by the full layout
/// scale factor every time.
///
/// [`UiGlobalTransform`]: bevy::ui::UiGlobalTransform
#[inline]
pub fn physical_to_ui_px(physical: Vec2, computed: &ComputedNode) -> Vec2 {
    physical * computed.inverse_scale_factor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_to_ui_px_is_identity_at_unit_scale() {
        let p = Vec2::new(120.0, 80.0);
        assert_eq!(window_to_ui_px(p, &UiScale(1.0)), p);
    }

    #[test]
    fn window_to_ui_px_shrinks_when_the_interface_is_scaled_up() {
        // At UiScale 1.5 a node placed at `Val::Px(n)` renders 1.5*n physical
        // px from the origin, so a cursor 300 window-px in must be written as
        // 200 to land under the pointer.
        assert_eq!(
            window_to_ui_px(Vec2::new(300.0, 150.0), &UiScale(1.5)),
            Vec2::new(200.0, 100.0)
        );
    }
}
