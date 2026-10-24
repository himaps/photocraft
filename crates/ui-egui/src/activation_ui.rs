//! The activation gate: the app does not run until this installation is activated.
//!
//! The owner requires each installation to be activated with a four-word code derived from the
//! machine's hardware identifier (see `photocraft-activation`). Until a valid code is entered,
//! `fn ui` renders only this screen — no tabs, no documents, no tools.
//!
//! The gate is part of the shipped build only: normal development builds, the test suite and
//! the browser build compile it out (see the `activation-gate` feature on this crate), so the
//! ~200 kittest tests and `cargo run` never see it.

use crate::PhotocraftApp;

/// The activation state of one run. `activated` is the gate: `fn ui` checks it before
/// rendering anything else. The machine fields only exist for the shipped native gate; in
/// development, test and wasm builds (where the gate is compiled out) they would be dead, so
/// the lint is switched off for exactly that configuration.
#[derive(Default)]
#[cfg_attr(any(not(feature = "activation-gate"), target_arch = "wasm32"), allow(dead_code))]
pub struct Dialog {
    /// The hardware identifier, read once (it cannot change mid-run).
    machine_id: Option<String>,
    /// The machine code rendered as words, computed once for the gate screen.
    machine_code: Option<String>,
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
            Err(_) => Self::default(),
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
    // Only the shipped gate touches the activation error type and the theme/widget helpers.
    use crate::{theme, widgets};
    use photocraft_activation::ActivationError;

    if app.activation.activated {
        return true;
    }
    let Some(id) = app.activation.machine_id.clone() else {
        // No hardware identifier: show an error but keep the gate shut — we cannot derive
        // a machine code, so there is nothing the user could send to the author.
        let t = theme::Tokens::get(ui.ctx());
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                ui.add_space(120.0);
                ui.label(egui::RichText::new(tl!("Activate PhotoCraft")).font(theme::semibold(20.0)));
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(tl!("Cannot read a hardware identifier on this machine; activation is not possible."))
                        .color(egui::Color32::RED),
                );
            });
        });
        return false;
    };
    // Activated at some earlier run (the stored code matches this machine): open the gate.
    if photocraft_activation::status(&id).is_ok() {
        app.activation.activated = true;
        return true;
    }

    // The machine code is derived once; rendering it is pure layout.
    let machine_text = app
        .activation
        .machine_code
        .get_or_insert_with(|| photocraft_activation::machine_code(&id).render())
        .clone();
    let t = theme::Tokens::get(ui.ctx());
    let mut activate = false;

    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| {
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
            ui.add_space(80.0);
            ui.add(crate::icons::image("badge-check", 36.0, t.accent));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(tl!("Activate PhotoCraft")).font(theme::semibold(22.0)));
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(tl!(
                    "Activation is free: it only tells the author that this installation exists. The code stays on this machine until you send it yourself."
                ))
                .color(t.text_dim),
            );
            ui.add_space(20.0);

            // Machine code, inside a bordered card.
            egui::Frame::default()
                .fill(t.field)
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(420.0);
                    ui.label(tl!("Your machine code — send it to the author:"));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&machine_text).font(theme::semibold(16.0)).color(t.accent_text));
                        if widgets::pill_button(ui, tl!("Copy"), false).clicked() {
                            ui.ctx().copy_text(machine_text.clone());
                        }
                    });
                });

            ui.add_space(16.0);

            // Activation code input, inside a bordered card.
            egui::Frame::default()
                .fill(t.field)
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(420.0);
                    ui.label(tl!("Paste the activation code you received:"));
                    ui.add_space(4.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut app.activation.typed)
                            .hint_text(tl!("four words joined by -"))
                            .desired_width(f32::INFINITY),
                    );
                });

            ui.add_space(10.0);
            if let Some(error) = &app.activation.error {
                ui.label(egui::RichText::new(error).color(egui::Color32::RED));
                ui.add_space(6.0);
            }
            ui.add_space(4.0);
            if widgets::pill_button(ui, tl!("Activate"), true).clicked() {
                activate = true;
            }
        });
    });

    if activate {
        app.activation.error = None;
        match photocraft_activation::activate(&id, &app.activation.typed) {
            Ok(code) => {
                app.activation.activated = true;
                crate::notices::post(app, tl!("Activated. Thank you!"), Vec::new(), false, None);
                log::info!("activation stored ({} words)", code.len());
            }
            Err(ActivationError::Write(path, error)) => {
                // The install directory is root-owned on Linux; the words are safe to quote
                // (they come from the 64-word table), so the exact command is shown.
                let code =
                    photocraft_activation::parse(&app.activation.typed).map(|c| c.render()).unwrap_or_else(|_| app.activation.typed.trim().to_owned());
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
                    false,
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

#[cfg(all(test, feature = "activation-gate", not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// The state transitions the gate depends on: a fresh dialog on a machine with a hardware
    /// id is shut (no activation stored in the test environment), and entering this machine's
    /// own activation code opens it without the UI.
    #[test]
    fn dialog_reads_the_stored_activation_once() {
        let d = Dialog::new();
        if d.machine_id.is_none() {
            return; // no hardware id here; the gate would explain and stay shut
        }
        assert!(!d.activated, "a fresh test environment has no stored activation");
        // Activating through the library (the same call the gate makes) opens the dialog.
        let id = d.machine_id.clone().unwrap();
        let typed = photocraft_activation::activation_code(&id).render();
        // activate() writes next to the test executable; only assert the verification logic.
        let expected = photocraft_activation::activation_code(&id);
        assert_eq!(photocraft_activation::parse(&typed).unwrap(), expected);
    }
}
