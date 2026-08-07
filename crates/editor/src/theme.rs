//! Editor visual theme.
//!
//! Centralises every color, spacing and stroke used by the chrome so the
//! editor looks like a small game-engine (dark panels, accent on the active
//! tool, subtle separators) instead of the default egui demo skin.
//!
//! All colors are defined in sRGB and converted to linear/`Color32` at use
//! sites. Keep this module free of `bevy_egui` types so it can be unit-tested
//! or reused by a future screenshot tool.

use bevy_egui::egui::{self, Color32, CornerRadius, Stroke, Vec2};

/// Accent color used for the active tool, focused inputs and key headers.
///
/// Picked as a warm orange (`#FF8C42`) reminiscent of the Unity/Blender
/// selection highlight, which stays readable on the dark panel fill.
pub const ACCENT: [u8; 3] = [0xFF, 0x8C, 0x42];

/// Secondary accent used for hover states (cyan-ish, like Unreal).
pub const ACCENT_SOFT: [u8; 3] = [0x4F, 0x9C, 0xFF];

/// Success / saved indicator (green).
pub const SUCCESS: [u8; 3] = [0x6F, 0xC7, 0x97];

/// Warning / unsaved indicator (yellow).
pub const WARNING: [u8; 3] = [0xE3, 0xB3, 0x4A];

/// Error / validation issue indicator (red).
pub const ERROR: [u8; 3] = [0xE5, 0x6B, 0x6B];

/// Palette of engine-style panel fills and strokes.
pub struct EditorPalette {
    pub panel_bg: Color32,
    pub panel_bg_alt: Color32,
    pub toolbar_bg: Color32,
    pub separator: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
}

impl EditorPalette {
    /// Dark engine palette. Hard-coded (no runtime theming yet) so all panels
    /// stay visually consistent.
    pub fn dark() -> Self {
        Self {
            panel_bg: Color32::from_rgb(0x22, 0x24, 0x28),
            panel_bg_alt: Color32::from_rgb(0x2A, 0x2D, 0x33),
            toolbar_bg: Color32::from_rgb(0x1A, 0x1C, 0x20),
            separator: Color32::from_rgb(0x0E, 0x0F, 0x12),
            text: Color32::from_rgb(0xE6, 0xE8, 0xEB),
            text_dim: Color32::from_rgb(0x9A, 0x9E, 0xA6),
            accent: Color32::from_rgb(ACCENT[0], ACCENT[1], ACCENT[2]),
            accent_soft: Color32::from_rgb(ACCENT_SOFT[0], ACCENT_SOFT[1], ACCENT_SOFT[2]),
            success: Color32::from_rgb(SUCCESS[0], SUCCESS[1], SUCCESS[2]),
            warning: Color32::from_rgb(WARNING[0], WARNING[1], WARNING[2]),
            error: Color32::from_rgb(ERROR[0], ERROR[1], ERROR[2]),
        }
    }
}

/// Applies the dark engine theme to an egui context.
///
/// Idempotent: safe to call every frame, only mutates visuals/style.
pub fn apply(ctx: &egui::Context) {
    let p = EditorPalette::dark();

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = p.panel_bg;
    visuals.window_fill = p.panel_bg_alt;
    visuals.faint_bg_color = p.toolbar_bg;
    visuals.extreme_bg_color = Color32::from_rgb(0x12, 0x13, 0x16);
    visuals.hyperlink_color = p.accent_soft;
    visuals.selection.bg_fill = p.accent;
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.noninteractive.bg_fill = p.panel_bg;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.separator);
    visuals.widgets.inactive.bg_fill = p.panel_bg_alt;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.separator);
    visuals.widgets.hovered.bg_fill = p.panel_bg_alt;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.accent_soft);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent_soft);
    visuals.widgets.active.bg_fill = p.accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.open.bg_fill = p.panel_bg_alt;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, p.accent);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = Vec2::new(6.0, 4.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.window_margin = style.spacing.window_margin * 1.5;
    style.spacing.menu_margin = egui::Margin::same(6);
    style.visuals.button_frame = true;
    style.spacing.tooltip_width = 240.0;
    ctx.set_global_style(style);
}

/// Frame used by the dock-style side panels: subtle bottom separator, no
/// rounding, opaque background so the 3D viewport never bleeds through.
pub fn panel_frame(palette: &EditorPalette) -> egui::Frame {
    egui::Frame::NONE
        .fill(palette.panel_bg)
        .stroke(Stroke::new(1.0_f32, palette.separator))
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::same(8))
}

/// Frame used by the thin top toolbar / status bar.
pub fn bar_frame(palette: &EditorPalette) -> egui::Frame {
    egui::Frame::NONE
        .fill(palette.toolbar_bg)
        .stroke(Stroke::new(1.0_f32, palette.separator))
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(8, 4))
}

/// Returns an accent-tinted header text.
pub fn heading(text: &str, palette: &EditorPalette) -> egui::RichText {
    egui::RichText::new(text)
        .strong()
        .color(palette.accent)
        .size(13.0)
}

/// Returns a small dim caption (used for hotkey hints and inline notes).
pub fn caption(text: &str, palette: &EditorPalette) -> egui::RichText {
    egui::RichText::new(text).color(palette.text_dim).size(11.0)
}
