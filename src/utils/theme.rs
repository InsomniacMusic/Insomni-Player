use egui::{Color32, Rounding, Stroke, Vec2, Visuals};

pub const ACCENT: Color32 = Color32::from_rgb(220, 220, 228);
pub const BG: Color32 = Color32::from_rgb(13, 13, 13);
pub const SURFACE: Color32 = Color32::from_rgb(22, 22, 22);
pub const SURFACE2: Color32 = Color32::from_rgb(32, 32, 32);
pub const TEXT_DIM: Color32 = Color32::from_rgb(100, 100, 110);
pub const TEXT_MID: Color32 = Color32::from_rgb(170, 170, 180);

pub fn apply_theme(ctx: &egui::Context) {
    let mut v = Visuals::dark();
    v.override_text_color = Some(Color32::from_rgb(210, 210, 215));
    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = SURFACE;
    v.faint_bg_color = SURFACE;
    v.widgets.noninteractive.bg_fill = SURFACE;
    v.widgets.noninteractive.rounding = Rounding::same(4.0);
    v.widgets.inactive.bg_fill = SURFACE2;
    v.widgets.inactive.rounding = Rounding::same(4.0);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_MID);
    v.widgets.hovered.bg_fill = Color32::from_rgb(42, 42, 48);
    v.widgets.hovered.rounding = Rounding::same(4.0);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.active.bg_fill = Color32::from_rgb(30, 30, 38);
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.selection.bg_fill = Color32::from_rgb(30, 30, 38);
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.window_rounding = Rounding::same(6.0);
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    ctx.set_style(style);
}
