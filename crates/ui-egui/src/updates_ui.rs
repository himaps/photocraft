//! Help ▸ Check for Updates and the check at startup, both driven by craft-update's `Updater`.
//!
//! This fork never checks for updates over the network: the update feed is the upstream
//! project's GitHub releases, which is not where this fork's installers come from, and the
//! request at start is one a security tool rightly asks about. The checker keeps its dialogs
//! and its "last update" record, and answers every check with [`OFFLINE`] instead of reaching
//! the network (see [`updater`]).

use serde_json::Value;

use crate::PhotocraftApp;

/// What every update check answers on this fork: no release, and the reason.
#[cfg(not(target_arch = "wasm32"))]
const OFFLINE: &str = "This build does not check for updates over the network: new versions come from the project's releases page.";

/// The check source this fork uses: answers [`OFFLINE`] and opens no connection. The framework
/// reaches the network only when a checker has no source, so every checker gets this one.
#[cfg(not(target_arch = "wasm32"))]
fn offline_source() -> craft::update::updater::CheckSource {
    std::sync::Arc::new(|| Err(OFFLINE.to_owned()))
}

/// The update checker, wired never to reach the network ([`offline_source`]). Every `Updater`
/// the app builds goes through here.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn updater() -> craft::update::updater::Updater {
    craft::update::updater::Updater::new(&crate::UPDATE_APP, photocraft_engine::build_info::VERSION).with_source(offline_source())
}

/// The update checker's state (desktop); the web build has none.
pub(crate) struct Runtime {
    /// The one checker: the startup prompt, the manual check, the installation and the last update's outcome.
    #[cfg(not(target_arch = "wasm32"))]
    updater: craft::update::updater::Updater,
    /// The check-result dialog on the dialog host, while one is open.
    #[cfg(not(target_arch = "wasm32"))]
    result_dialog: Option<craft::ui::dialog::DialogId>,
    /// The last update's outcome opened the result window; it opens once, so closing it with `ui.set` stays closed.
    #[cfg(not(target_arch = "wasm32"))]
    outcome_announced: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            updater: updater(),
            #[cfg(not(target_arch = "wasm32"))]
            result_dialog: None,
            #[cfg(not(target_arch = "wasm32"))]
            outcome_announced: false,
        }
    }
}

/// Help ▸ Check for Updates: starts a check whose outcome the result window shows.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn check(app: &mut PhotocraftApp, _ctx: &egui::Context, _params: Value) -> Result<Value, String> {
    app.updates_rt.updater.check_now();
    app.ui.updates_open = true;
    Ok(serde_json::json!({"status": "checking"}))
}

/// Update checks need a desktop install.
#[cfg(target_arch = "wasm32")]
pub(crate) fn check(_app: &mut PhotocraftApp, _ctx: &egui::Context, _params: Value) -> Result<Value, String> {
    Err("Update checks are not available here".into())
}

/// Once per pass: the check at start when the preference is on (the `Updater` runs it once per
/// launch), else only the read of the last update's outcome (also once, inside the `Updater`).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn tick(app: &mut PhotocraftApp) {
    use craft::update::updater::Status;
    if app.session.prefs().general.automatically_check_updates {
        app.updates_rt.updater.check_at_start();
    } else {
        app.updates_rt.updater.read_last_update();
    }
    if !app.updates_rt.outcome_announced && matches!(app.updates_rt.updater.status(), Status::Updated(_) | Status::InstallFailed(_)) {
        app.updates_rt.outcome_announced = true;
        app.ui.updates_open = true;
    }
}

/// The web build has no checker.
#[cfg(target_arch = "wasm32")]
pub(crate) fn tick(_app: &mut PhotocraftApp) {}

/// The web build has no checker.
#[cfg(target_arch = "wasm32")]
pub(crate) fn show(_app: &mut PhotocraftApp, _ctx: &egui::Context) {}

/// Drives the updater's own prompt and the check-result dialog on the framework dialog host. Runs
/// before `DialogHost::run`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn show(app: &mut PhotocraftApp, ctx: &egui::Context) {
    use craft::update::stack_dialogs::{CheckChoice, UpdateCheckDialog};
    use craft::update::updater::{Action, Status};

    // An update starts, and a ready one asks the app to close, only while no work is unsaved.
    let clean = !app.has_unsaved_work();
    app.updates_rt.updater.tick(ctx);
    match app.updates_rt.updater.run(&mut app.framework.dialogs, clean) {
        Action::None => {}
        Action::OpenUrl(url) => {
            let _ = crate::links::open(app, ctx, &url);
        }
        // The guarded close path asks about unsaved work and waits for saves; the restart helper
        // relaunches once the process exits.
        Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        _ => {}
    }

    // A cancelled result (the dialog withdrawn below) answers nothing.
    if let Some(choice) = app.framework.dialogs.take::<UpdateCheckDialog>().and_then(|result| result.ok()) {
        app.updates_rt.result_dialog = None;
        let updater = &mut app.updates_rt.updater;
        match choice {
            CheckChoice::CheckNow | CheckChoice::CheckAgain => updater.check_now(),
            CheckChoice::RestartToUpdate => updater.restart_to_update(),
            CheckChoice::Close => {
                updater.dismiss_status();
                app.ui.updates_open = false;
            }
            _ => {}
        }
    }

    // An available release shows in the updater's prompt instead of the result window.
    let updater = &app.updates_rt.updater;
    let wanted = app.ui.updates_open && *updater.status() != Status::Idle && !updater.prompt_open();
    let status = updater.status().clone();
    let dialogs = &mut app.framework.dialogs;
    if wanted {
        if let Ok(id) = dialogs.open_unless_open(UpdateCheckDialog::new(photocraft_engine::build_info::VERSION)) {
            app.updates_rt.result_dialog = Some(id);
        }
        if let Some(dialog) = app.updates_rt.result_dialog.and_then(|id| dialogs.get_mut_by_id::<UpdateCheckDialog>(id)) {
            dialog.set_status(status);
        }
    } else if let Some(id) = app.updates_rt.result_dialog.take() {
        dialogs.cancel_by_id(ctx, id);
    }
}

/// The updater's installation progress or error line, for `ui.inspect`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn status_message(app: &PhotocraftApp) -> Option<String> {
    app.updates_rt.updater.status_message()
}

/// The web build has no updater, so no message.
#[cfg(target_arch = "wasm32")]
pub(crate) fn status_message(_app: &PhotocraftApp) -> Option<String> {
    None
}

/// Offers `version` as an available release, as a check at start would, for the snapshot example.
#[cfg(not(target_arch = "wasm32"))]
pub fn preview_release(app: &mut PhotocraftApp, version: &str, url: &str) {
    app.updates_rt.updater.offer(craft::update::Release {
        tag: format!("v{version}"),
        version: craft::update::parse_version(version),
        name: format!("PhotoCraft {version}"),
        html_url: url.into(),
        ..Default::default()
    });
    app.ui.updates_open = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Services, menus};
    use photocraft_engine::Session;
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;

    fn app() -> Box<PhotocraftApp> {
        PhotocraftApp::new(Session::new(), Services::default())
    }

    /// Runs the app's per-pass update handling (`tick` then `show`, as a frame does) until the
    /// checker's running check lands, then returns its status. The checker picks up its check
    /// thread's answer in `show`, as in the app.
    #[cfg(not(target_arch = "wasm32"))]
    fn settled(app: &mut PhotocraftApp) -> craft::update::updater::Status {
        use craft::update::updater::Status;
        for _ in 0..200 {
            tick(app);
            show(app, &app.ctx().clone());
            let status = app.updates_rt.updater.status().clone();
            if !matches!(status, Status::Checking) {
                return status;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        app.updates_rt.updater.status().clone()
    }

    /// The app as shipped never reaches the network for updates. The framework opens a
    /// connection only for a checker without a source; the app's checker is built with the
    /// offline one, which answers without connecting.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn the_apps_checker_answers_offline_instead_of_connecting() {
        use craft::update::updater::Status;
        assert!(matches!(offline_source()(), Err(reason) if reason == OFFLINE));
        let mut app = app();
        // The check at start, with its preference on (the default): a quiet check that finds
        // nothing stays quiet, so nothing shows and the status stays idle.
        app.session.edit_prefs(|p| p.general.automatically_check_updates = true);
        tick(&mut app);
        assert_eq!(settled(&mut app), Status::Idle, "the check at start shows nothing");
        assert!(!app.updates_rt.updater.prompt_open(), "no update prompt opens at start");
        // Help ▸ Check for Updates: the result window says why there is no answer.
        let ctx = app.ctx().clone();
        menus::invoke(&mut app, &ctx, "help.checkForUpdates", serde_json::json!({})).unwrap();
        assert_eq!(settled(&mut app), Status::Failed(OFFLINE.to_owned()), "a requested check reports the offline answer");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn manual_check_works_when_automatic_checks_are_off_and_opens_status() {
        let (started, checks) = mpsc::channel();
        let started = Mutex::new(started);
        let source: craft::update::updater::CheckSource = Arc::new(move || {
            let _ = started.lock().unwrap().send(());
            Err("offline".into())
        });
        let mut app = app();
        app.updates_rt.updater = craft::update::updater::Updater::new(&crate::UPDATE_APP, photocraft_engine::build_info::VERSION).with_source(source);
        app.session.edit_prefs(|p| p.general.automatically_check_updates = false);

        tick(&mut app);
        assert!(checks.recv_timeout(Duration::from_millis(100)).is_err(), "no automatic check while the preference is off");
        assert_eq!(*app.updates_rt.updater.status(), craft::update::updater::Status::Idle);

        let ctx = app.ctx().clone();
        menus::invoke(&mut app, &ctx, "help.checkForUpdates", serde_json::json!({})).unwrap();
        assert!(app.ui.updates_open);
        assert_ne!(*app.updates_rt.updater.status(), craft::update::updater::Status::Idle);
        assert!(checks.recv_timeout(Duration::from_secs(5)).is_ok(), "the manual check ran");
    }

    #[test]
    fn update_window_is_drivable_and_bad_state_changes_are_atomic() {
        let mut app = app();
        let ctx = app.ctx().clone();
        let call = |app: &mut PhotocraftApp, method: &str, params: Value| {
            let (req, _) = crate::control::ControlRequest::new(method, params);
            match crate::control::handle(app, &ctx, &req) {
                crate::control::Reply::Done(value) => value,
                _ => panic!("expected an immediate reply"),
            }
        };
        assert_eq!(call(&mut app, "ui.set", serde_json::json!({"updatesOpen": true}))["ok"], true);
        let inspected = call(&mut app, "ui.inspect", serde_json::json!({}));
        assert_eq!(inspected["result"]["updatesOpen"], true);
        assert_eq!(inspected["result"]["updates"]["supported"], true);
        assert_eq!(call(&mut app, "ui.set", serde_json::json!({"updatesOpen": false, "maskTarget": "bad"}))["ok"], false);
        assert!(app.ui.updates_open);
        assert_eq!(call(&mut app, "ui.set", serde_json::json!({"updatesOpen": false}))["ok"], true);
        assert!(!app.ui.updates_open);
    }
}
