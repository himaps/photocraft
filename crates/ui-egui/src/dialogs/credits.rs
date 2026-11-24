//! The About window: name, blurb, version. This fork shows no contributor or model tables and
//! no community links, so the credits are passed empty and build.rs's baked tables stay unused.

use craft::ui::dialog::DialogId;
use craft::ui::patterns::form_dialogs::{AboutInfo, AboutTab, Credits, HostedAbout, HostedReport};

use crate::PhotocraftApp;

/// The About tab's contents: name, blurb, version.
pub fn about_info(_app: &PhotocraftApp) -> AboutInfo {
    AboutInfo::new("PhotoCraft", tl!("PhotoCraft — an open-source, native image editor written in Rust."), photocraft_engine::build_info::long_version())
        .built_with("egui · wgpu · photocraft-engine")
}

/// Opens the About dialog on `tab`.
pub fn open_about(app: &mut PhotocraftApp, tab: AboutTab) -> DialogId {
    // The contributor and model tables are not shown on this fork: empty credits.
    let dialog = HostedAbout::new(about_info(app), Credits::new(Vec::new(), Vec::new(), 0)).tab(tab);
    app.framework.dialogs.open(dialog)
}

/// Opens Help › System Info: the version line as the summary, the platform and graphics facts as lines.
pub fn open_system_info(app: &mut PhotocraftApp) -> DialogId {
    let mut lines = crate::gpu_status::system_info_ui(app);
    let summary = if lines.is_empty() { String::new() } else { lines.remove(0) };
    app.framework.dialogs.open(HostedReport::new(tl!("System Info"), summary, lines))
}

/// The result pump (a `TAKERS` row): opens a link the About dialog took and copies System Info. Each call takes only
/// what the dialogs filed since the last call.
pub(crate) fn take(app: &mut PhotocraftApp, ctx: &egui::Context) {
    if let Some(url) = app.framework.dialogs.get_mut::<HostedAbout>().and_then(HostedAbout::take_link) {
        crate::links::open(app, ctx, &url);
    }
    // Issue reports take the English lines whatever the interface language, as the Copy button always has.
    if app.framework.dialogs.get_mut::<HostedReport>().and_then(HostedReport::take_copied).is_some() {
        ctx.copy_text(crate::gpu_status::system_info(app).join("\n"));
    }
}
