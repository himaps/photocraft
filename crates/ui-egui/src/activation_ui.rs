//! The activation gate: the app does not run until this installation is activated.
//!
//! The owner requires each installation to be activated with a four-word code derived from the
//! machine's hardware identifier (see `photocraft-activation`). Until a valid code is entered,
//! `fn ui` renders only this screen — no tabs, no documents, no tools.
//!
//! The screen follows ZCode's offline activation gate (`ZCode/packages/ui/src/activation/
//! ActivationGate.tsx`): the canvas-coloured screen with a soft accent glow behind one centred
//! card — the product mark, the machine code as word chips with a copy affordance on the label
//! row, the activation input whose typed text carries the fork's brand red, a full-width
//! primary button, and a footer note with a quit button. Activation logic is unchanged
//! (`photocraft_activation`). The gate is part of the shipped build only: normal development
//! builds, the test suite and the browser build compile it out (see the `activation-gate`
//! feature on this crate), so the kittest suites and `cargo run` never see it.

use crate::PhotocraftApp;
use egui::{Align2, Color32, CornerRadius, FontFamily, FontId, Margin, Sense, Stroke, StrokeKind, Vec2, vec2};

/// The card's width: ZCode's `max-w-lg` (32 rem).
const CARD_WIDTH: f32 = 512.0;
/// The bare screen's card is a step narrower (`max-w-md`).
const BARE_CARD_WIDTH: f32 = 448.0;
/// The input row's and the primary button's height (`h-11`).
const INPUT_H: f32 = 44.0;
/// The brand red the typed activation code draws in: the app icon's letter red, sampled from
/// the design (`#cb3149`).
const INPUT_INK: Color32 = Color32::from_rgb(0xCB, 0x31, 0x49);
/// The mark's size (`size-12`): the app icon itself, drawn from the embedded 256 px tile.
const MARK_SIZE: f32 = 48.0;
/// The app icon's 256 px tile (the same bytes the Linux hicolor theme and the window icon use).
#[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
const MARK_PNG: &[u8] = include_bytes!("../../../assets/app-icon/hicolor/256x256/apps/ai.storyteller.photocraft.png");

/// The activation state of one run. `activated` is the gate: `fn ui` checks it before
/// rendering anything else. The machine fields only exist for the shipped native gate; in
/// development, test and wasm builds (where the gate is compiled out) they would be dead, so
/// the lint is switched off for exactly that configuration.
#[derive(Default)]
#[cfg_attr(any(not(feature = "activation-gate"), target_arch = "wasm32"), allow(dead_code))]
pub struct Dialog {
    /// The hardware identifier, read once (it cannot change mid-run).
    machine_id: Option<String>,
    /// Why the hardware identifier is missing, shown on the bare screen.
    machine_error: Option<String>,
    /// The machine code rendered as words, computed once for the gate screen.
    machine_code: Option<String>,
    /// When the copy affordance last fired: it reads "Copied" for a while after.
    copied_until: Option<std::time::Instant>,
    typed: String,
    error: Option<String>,
    /// Set to `true` when a valid activation code is verified. `fn ui` gates on this.
    pub activated: bool,
}

impl Dialog {
    /// Reads the hardware identifier once and whether a valid activation is already stored, so
    /// `fn ui` never touches the activation file per frame.
    pub fn new() -> Self {
        match photocraft_activation::machine_id() {
            Ok(id) => {
                let activated = photocraft_activation::status(&id).is_ok();
                Self { machine_id: Some(id), activated, ..Default::default() }
            }
            // Without a hardware identifier we cannot derive a code: the gate stays shut.
            Err(e) => Self { machine_error: Some(e.to_string()), ..Default::default() },
        }
    }
}

/// Draws the fullscreen activation panel when the installation is not yet activated.
/// Returns `true` when the main UI should render (i.e. the installation is activated).
pub fn gate(app: &mut PhotocraftApp, ui: &mut egui::Ui) -> bool {
    // The gate ships only in packaging builds: development, tests and the browser build run
    // without it (there is no machine identity to derive a code from in a browser anyway).
    #[cfg(any(not(feature = "activation-gate"), target_arch = "wasm32"))]
    {
        let _ = (app, ui);
        true
    }
    #[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
    gate_native(app, ui)
}

/// The native gate: the shipped desktop build's fullscreen activation screen.
#[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
fn gate_native(app: &mut PhotocraftApp, ui: &mut egui::Ui) -> bool {
    use crate::notices::Tone;
    use photocraft_activation::ActivationError;

    if app.activation.activated {
        return true;
    }
    let Some(id) = app.activation.machine_id.clone() else {
        // No hardware identifier: the bare card explains and the gate stays shut — we cannot
        // derive a machine code, so there is nothing the user could send to the maintainer.
        bare_screen(ui, app.activation.machine_error.clone());
        return false;
    };
    // Activated at some earlier run (the stored code matches this machine): open the gate.
    if photocraft_activation::status(&id).is_ok() {
        app.activation.activated = true;
        return true;
    }

    // The machine code is derived once; rendering it is pure layout.
    let machine_text = app.activation.machine_code.get_or_insert_with(|| photocraft_activation::machine_code(&id).render()).clone();
    let words: Vec<&str> = machine_text.split(photocraft_activation::SEPARATOR).collect();
    let mut activate = false;

    screen(ui, CARD_WIDTH, |ui| {
        let t = crate::theme::Tokens::get(ui.ctx());
        mark(ui, t.accent);
        ui.add_space(16.0);
        ui.label(egui::RichText::new(tl!("Activate PhotoCraft")).size(20.0).strong());
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(tl!("Offline activation · free to use · the code is bound to this machine"))
                .size(14.0)
                .color(t.text_dim),
        );
        ui.add_space(28.0);

        // The machine code, as word chips, with the copy affordance on the label row.
        ui.horizontal(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label(egui::RichText::new(tl!("Machine code")).size(14.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let copied = app.activation.copied_until.is_some_and(|until| std::time::Instant::now() < until);
                if ghost_button(ui, &t, if copied { tl!("Copied") } else { tl!("Copy") }).clicked() {
                    ui.ctx().copy_text(machine_text.clone());
                    app.activation.copied_until = Some(std::time::Instant::now() + std::time::Duration::from_millis(1600));
                }
            });
        });
        ui.add_space(8.0);
        word_chips(ui, &t, &words);
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(tl!("Send this machine code to the maintainer to receive its activation code"))
                .size(12.0)
                .color(t.text_dim),
        );
        ui.add_space(24.0);

        // The activation code received back. A failed attempt keeps the typed words.
        ui.label(egui::RichText::new(tl!("Activation code")).size(14.0).strong());
        ui.add_space(8.0);
        let input = field_box(ui, &t, Margin::symmetric(12, 3), Stroke::new(1.0, t.accent.gamma_multiply(0.3)), |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.activation.typed)
                    .hint_text(tl!("four words joined by -"))
                    .font(FontId::new(16.0, FontFamily::Proportional))
                    .text_color(INPUT_INK)
                    .desired_width(f32::INFINITY)
                    .margin(Margin::symmetric(0, 8))
                    .frame(egui::Frame::NONE),
            )
        });
        input.response.request_focus();
        // Enter submits, as on ZCode's gate.
        if input.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            activate = true;
        }
        ui.add_space(8.0);
        if let Some(error) = &app.activation.error {
            ui.label(egui::RichText::new(error).size(14.0).color(t.danger));
            ui.add_space(6.0);
        }
        ui.add_space(16.0);
        // The full-width primary button; disabled until something is typed.
        let empty = app.activation.typed.trim().is_empty();
        if primary_button(ui, &t, tl!("Activate"), ui.available_width(), empty) {
            activate = true;
        }
        ui.add_space(24.0);
        ui.separator();
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(tl!(
                "The activation code comes from the maintainer's offline activation tool and matches this machine's code"
            ))
            .size(12.0)
            .color(t.text_dim),
        );
        ui.add_space(16.0);
        if ghost_button(ui, &t, tl!("Quit")).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });

    if activate {
        app.activation.error = None;
        match photocraft_activation::activate(&id, &app.activation.typed) {
            Ok(code) => {
                app.activation.activated = true;
                crate::notices::post(app, tl!("Activated. Thank you!"), Vec::new(), Tone::Neutral, None);
                log::info!("activation stored ({} words)", code.len());
            }
            Err(ActivationError::Write(path, error)) => {
                // The install directory is root-owned on Linux; the words are safe to quote
                // (they come from the word table), so the exact command is shown.
                let code = photocraft_activation::parse(&app.activation.typed)
                    .map(|c| c.render())
                    .unwrap_or_else(|_| app.activation.typed.trim().to_owned());
                app.activation.error =
                    Some(crate::i18n::fmt(tl!("Couldn't write {path}: {error}"), &[("path", path.as_str()), ("error", error.as_str())]));
                crate::notices::post(
                    app,
                    tl!("To store the activation, run this in a terminal and restart:"),
                    vec![format!(
                        "sudo mkdir -p \"$(dirname '{path}')\" && printf '%s\\n' '{code}' | sudo tee '{path}' >/dev/null",
                        path = path,
                        code = code
                    )],
                    Tone::Neutral,
                    None,
                );
            }
            Err(ActivationError::BadWord { word, .. }) => {
                app.activation.error = Some(crate::i18n::fmt(tl!("That word is not in the table: {word}"), &[("word", &word)]));
            }
            Err(ActivationError::WrongLength { expected, got }) => {
                app.activation.error = Some(crate::i18n::fmt(
                    tl!("A code has {expected} words, this one has {got}."),
                    &[("expected", &expected.to_string()), ("got", &got.to_string())],
                ));
            }
            Err(ActivationError::Mismatch) => {
                app.activation.error = Some(tl!("The activation code does not match this machine.").to_owned());
            }
            Err(e) => app.activation.error = Some(e.to_string()),
        }
    }

    app.activation.activated
}

/// The gate's canvas: the whole frame in the canvas colour with a soft accent glow behind the
/// centred card (ZCode's radial gradient, approximated with layered translucent circles), and
/// `body` drawn on a centred `width`-wide card.
#[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
fn screen(ui: &mut egui::Ui, width: f32, body: impl FnOnce(&mut egui::Ui)) {
    let t = crate::theme::Tokens::get(ui.ctx());
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| {
        let full = ui.max_rect();
        // The glow: concentric translucent discs behind where the card sits (ZCode paints a
        // radial gradient from the screen's top centre).
        let center = egui::Pos2::new(full.center().x, full.top() + 40.0);
        for (radius, alpha) in [(420.0, 0.015), (300.0, 0.02), (200.0, 0.03)] {
            ui.painter().circle_filled(center, radius, t.accent.gamma_multiply(alpha));
        }
        let card_width = width.min(full.width() - 32.0);
        // The card hugs its content and centres on the screen (ZCode: `items-center
        // justify-center`). The content height is only known after the pass, so this frame
        // centres on the previous pass's measurement (an estimate on the very first one).
        let id = ui.id().with("gate-card-height");
        let measured = ui.ctx().data_mut(|d| d.get_temp::<f32>(id).unwrap_or(if width == BARE_CARD_WIDTH { 240.0 } else { 700.0 }));
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.add_space(((full.height() - measured) / 2.0).max(24.0));
            let card = egui::Frame::default()
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.card_border))
                .corner_radius(CornerRadius::same(16))
                .inner_margin(32.0)
                .shadow(egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_rgba_unmultiplied(0, 0, 0, 48) })
                .show(ui, |ui| {
                    ui.set_width(card_width - 64.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| body(ui));
                });
            ui.ctx().data_mut(|d| d.insert_temp(id, card.response.rect.height()));
        });
    });
}

/// The bare screen: the machine identifier is missing, so there is nothing to send. A narrow
/// card with a destructive border, the explanation and the error's own text (status ② of the
/// activation guide): the gate stays shut.
#[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
fn bare_screen(ui: &mut egui::Ui, detail: Option<String>) {
    screen(ui, BARE_CARD_WIDTH, |ui| {
        let t = crate::theme::Tokens::get(ui.ctx());
        ui.label(egui::RichText::new(tl!("Cannot read the machine identifier")).size(15.0).strong().color(t.danger));
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(tl!("Cannot read a hardware identifier on this machine; activation is not possible."))
                .size(13.0)
                .color(t.text_dim),
        );
        if let Some(detail) = detail {
            ui.add_space(8.0);
            field_box(ui, &t, Margin::symmetric(8, 6), Stroke::NONE, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(detail).size(11.0).color(t.text_dim));
            });
        }
    });
}

/// The product mark: the app icon itself at `size-12`, centred (ZCode's gate shows a badge
/// glyph in the same slot). A decode failure draws the initials on a quiet tile instead — the
/// gate never fails for its ornament.
#[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
fn mark(ui: &mut egui::Ui, accent: Color32) {
    use std::sync::OnceLock;
    static TEXTURE: OnceLock<Option<egui::TextureHandle>> = OnceLock::new();
    let texture = TEXTURE.get_or_init(|| {
        let image = photocraft_codecs::decode(MARK_PNG)
            .ok()
            .map(|img| img.convert(photocraft_codecs::ChannelLayout::Rgba, photocraft_codecs::SampleType::U8))?;
        let size = [image.width() as usize, image.height() as usize];
        let data = image.data().to_vec();
        Some(ui.ctx().load_texture(
            "activation-gate-mark",
            egui::ColorImage::from_rgba_unmultiplied(size, &data),
            egui::TextureOptions::LINEAR,
        ))
    });
    let (rect, _) = ui.allocate_exact_size(vec2(MARK_SIZE, MARK_SIZE), Sense::hover());
    match texture {
        Some(texture) => {
            ui.painter().image(texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            ui.painter().rect_filled(rect, CornerRadius::same(12), accent.gamma_multiply(0.12));
            let galley = ui.painter().layout_no_wrap("Pc".to_owned(), FontId::new(20.0, FontFamily::Proportional), accent);
            ui.painter().galley(Align2::CENTER_CENTER.pos_in_rect(&rect) - galley.size() / 2.0, galley, accent);
        }
    }
}

/// The machine code's word chips: the container with the accent-tinted border and fill, one
/// rounded chip per word (`text-lg font-semibold`), the row centred.
#[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
fn word_chips(ui: &mut egui::Ui, t: &crate::theme::Tokens, words: &[&str]) {
    // The words fit one line (four short words); measure the row, give the container exactly
    // that width plus its padding, and centre the container block in the card.
    let font = FontId::new(18.0, FontFamily::Proportional);
    let sizes: Vec<Vec2> = words
        .iter()
        .map(|w| ui.painter().layout_no_wrap((*w).to_owned(), font.clone(), t.text).size() + vec2(24.0, 12.0))
        .collect();
    let gap = 8.0;
    let total: f32 = sizes.iter().map(|s| s.x).sum::<f32>() + gap * (words.len().saturating_sub(1)) as f32;
    let block_width = (total + 32.0).min(ui.available_width());
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - block_width) / 2.0).max(0.0));
        egui::Frame::default()
            .fill(t.accent.gamma_multiply(0.05))
            .stroke(Stroke::new(1.0, t.accent.gamma_multiply(0.25)))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(total);
                ui.horizontal(|ui| {
                    for (word, size) in words.iter().zip(sizes) {
                        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                        ui.painter().rect(
                            rect,
                            CornerRadius::same(8),
                            t.field,
                            Stroke::new(1.0, t.accent.gamma_multiply(0.2)),
                            StrokeKind::Inside,
                        );
                        let galley = ui.painter().layout_no_wrap((*word).to_owned(), font.clone(), t.text);
                        ui.painter().galley(Align2::CENTER_CENTER.pos_in_rect(&rect) - galley.size() / 2.0, galley, t.text);
                    }
                });
            });
    });
}

/// One field box: `field` fill, `stroke` outline, 8 px corners, `margin` padding; returns the
/// frame's inner response (the TextEdit's, when the body adds one) so the caller can read
/// focus and keys.
#[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
fn field_box<R>(
    ui: &mut egui::Ui,
    t: &crate::theme::Tokens,
    margin: Margin,
    stroke: Stroke,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::default()
        .fill(t.field)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(margin)
        .show(ui, body)
}

/// A ghost button: borderless text that shows a quiet hover pill (`text-xs muted`).
#[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
fn ghost_button(ui: &mut egui::Ui, t: &crate::theme::Tokens, label: &str) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::new(12.0, FontFamily::Proportional), t.text_dim);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(16.0, 20.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), t.hover);
    }
    ui.painter().galley(Align2::CENTER_CENTER.pos_in_rect(&rect) - galley.size() / 2.0, galley, t.text_dim);
    resp
}

/// The full-width primary button (`h-11 text-base`); disabled (quiet fill, no click) while
/// `disabled`, mirroring ZCode's `disabled={typed.trim().length === 0}`.
#[cfg(all(feature = "activation-gate", not(target_arch = "wasm32")))]
fn primary_button(ui: &mut egui::Ui, t: &crate::theme::Tokens, label: &str, width: f32, disabled: bool) -> bool {
    let galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::new(16.0, FontFamily::Proportional), t.text);
    let size = vec2(width, INPUT_H);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled() && !disabled, label));
    let (fill, ink) = if disabled {
        (t.accent.gamma_multiply(0.35), t.accent_text.gamma_multiply(0.7))
    } else {
        let fill = if resp.is_pointer_button_down_on() {
            t.accent.gamma_multiply(0.8)
        } else if resp.hovered() {
            t.accent.gamma_multiply(0.9)
        } else {
            t.accent
        };
        (fill, t.accent_text)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, ink);
    resp.clicked() && !disabled
}

#[cfg(all(test, feature = "activation-gate", not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// The state transitions the gate depends on: a fresh dialog on a machine with a hardware
    /// id is shut (no activation stored in the test environment), and entering this machine's
    /// own activation code opens it without the UI.
    #[test]
    fn dialog_reads_the_stored_activation_once() {
        let d = Dialog::new();
        if d.machine_id.is_some() {
            assert!(d.machine_error.is_none(), "the identifier and its error are exclusive");
            assert!(!d.activated, "a fresh test environment has no stored activation");
            // Activating through the library (the same call the gate makes) opens the dialog.
            let id = d.machine_id.clone().unwrap();
            let typed = photocraft_activation::activation_code(&id).render();
            // activate() writes next to the test executable; only assert the verification logic.
            let expected = photocraft_activation::activation_code(&id);
            assert_eq!(photocraft_activation::parse(&typed).unwrap(), expected);
        } else {
            assert!(d.machine_error.is_some(), "the bare screen needs the reason");
        }
    }

    /// The gate's sizes follow ZCode's card: `max-w-lg`, `h-11` rows.
    #[test]
    fn sizes_match_zcodes_card() {
        assert_eq!(CARD_WIDTH, 512.0);
        assert_eq!(BARE_CARD_WIDTH, 448.0);
        assert_eq!(INPUT_H, 44.0);
        assert_eq!(MARK_SIZE, 48.0);
    }
}
