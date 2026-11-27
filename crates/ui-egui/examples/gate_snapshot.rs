//! Render the activation gate offscreen into a PNG, for design checks of the shipped gate
//! (the gate is compiled out of the default build):
//!
//! ```sh
//! cargo run -p photocraft-ui-egui --example gate_snapshot --features activation-gate -- \
//!     --out ../../target/activation_gate.png --size 1440x900 [--lang en]
//! ```
//!
//! `--lang` picks the interface language (default `en`: the test build installs no CJK font,
//! so the system language's glyphs would be missing). The gate reads this machine's hardware
//! identifier; with none, the render shows the bare "cannot activate" screen instead.

use photocraft_ui_egui::PhotocraftApp;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|at| args.get(at + 1)).cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = arg(&args, "--out").unwrap_or_else(|| "activation_gate.png".to_owned());
    let size = arg(&args, "--size").unwrap_or_else(|| "1440x900".to_owned());
    let (width, height) = size.split_once('x').map_or((1440.0, 900.0), |(w, h)| (w.parse().unwrap_or(1440.0), h.parse().unwrap_or(900.0)));
    let lang = arg(&args, "--lang").unwrap_or_else(|| "en".to_owned());

    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
    app.session.prefs.edit(|prefs| prefs.interface.language = lang.clone());
    // The example bypasses `ui_frame`, which sets the interface language every frame: set it here.
    photocraft_ui_egui::i18n::set_current(photocraft_ui_egui::i18n::Lang::from_code(&lang).unwrap_or(photocraft_ui_egui::i18n::Lang::EN));
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(width, height)).with_max_steps(64).with_context(app.ctx().clone()).build_ui_state(
        |ui, app: &mut Box<PhotocraftApp>| {
            photocraft_ui_egui::activation_ui::gate(app, ui);
        },
        app,
    );
    PhotocraftApp::setup_context(&h.ctx, Default::default());
    // The machine code renders as words of the activation table, which is Chinese-derived; the
    // shipped build draws it with the craft fonts, which a dev checkout has none of. Best effort:
    // add a platform CJK font when one exists, so the snapshot shows the real words.
    // The machine code renders as words of the activation table, which is Chinese-derived; the
    // shipped build draws them with the craft fonts. By default this matches a release: only the
    // embedded craft fonts (system font fallback is off). `--system-font` additionally adds a
    // platform CJK face, to see how the gate looks when one is present.
    #[cfg(target_os = "windows")]
    if arg(&args, "--system-font").is_some() {
        for file in ["msyh.ttc", "Deng.ttf", "simsun.ttc"] {
            let Ok(font) = std::fs::read(format!("C:/Windows/Fonts/{file}")) else { continue };
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert("cjk".to_owned(), std::sync::Arc::new(egui::FontData::from_owned(font)));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(family).or_default().push("cjk".to_owned());
            }
            h.ctx.set_fonts(fonts);
            break;
        }
    }
    h.run_steps(4);
    let image = h.render().expect("the gate renders");
    image.save(&out).expect("the gate PNG writes");
    println!("{} ({} × {}, interface language {lang})", out, width, height);

    // Diagnostics: whether this build embedded the craft-i18n catalogs (the language can only
    // switch when it did). The chip geometry is read from the PNG itself.
    if args.iter().any(|a| a == "--diag") {
        println!("--diag-- i18n catalogs embedded: {}", photocraft_ui_egui::i18n::catalogs_embedded());
    }
}
