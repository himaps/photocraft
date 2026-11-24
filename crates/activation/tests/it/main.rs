//! End to end: where the activation is stored and how it is read back. Every test works on its
//! own temp folder through the public functions, so none touches the real home directory or the
//! process environment.

use photocraft_activation::{self as act, ActivationError};
use std::path::{Path, PathBuf};

/// A private folder for one test, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("photocraft-activation-it-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Storing the code creates the folder when it does not exist yet (a profile that never ran the
/// app), the file keeps its two-line format, and the status agrees afterwards.
#[test]
fn a_fresh_profile_is_activated_by_storing_the_code() {
    let scratch = Scratch::new("fresh");
    let stored = scratch.0.join(".photocraft").join(".license.md");
    let id = act::machine_id().unwrap();

    assert!(matches!(act::status_migrating(None, &stored, id.as_str()), Err(ActivationError::NotActivated)), "nothing stored yet");
    assert!(!scratch.0.exists(), "asking the status writes nothing");

    act::activate_at(&stored, &id, &act::activation_code(&id).render()).unwrap();

    assert!(act::status_migrating(None, &stored, &id).is_ok(), "activated after storing");
    let text = std::fs::read_to_string(&stored).unwrap();
    assert!(text.starts_with("# photocraft\n- 激活码: "), "the file format is unchanged: {text:?}");
}

/// A code for another machine is refused and nothing is stored: not the file, not its folder.
#[test]
fn a_wrong_code_stores_nothing() {
    let scratch = Scratch::new("wrong");
    let stored = scratch.0.join(".photocraft").join(".license.md");
    let id = act::machine_id().unwrap();
    assert!(matches!(act::activate_at(&stored, &id, "富强-民主-文明-和谐"), Err(ActivationError::Mismatch)));
    assert!(!scratch.0.exists(), "a refused code writes nothing");
}

/// A machine activated by an earlier version (the code beside the installed files) is still
/// activated after upgrading, and the code then lives in the user's folder.
#[test]
fn an_earlier_install_directory_activation_still_counts() {
    let scratch = Scratch::new("upgrade");
    let legacy: PathBuf = scratch.0.join("usr").join("share").join("photocraft").join(".license.md");
    let stored: PathBuf = scratch.0.join("home").join(".photocraft").join(".license.md");
    std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    let id = act::machine_id().unwrap();
    std::fs::write(&legacy, format!("# {}\n- 激活码: {}\n", act::APP_ID, act::activation_code(&id).render())).unwrap();

    assert!(act::status_migrating(Some(legacy.as_path()), &stored, &id).is_ok(), "the machine stays activated");
    assert!(stored.is_file() && !legacy.exists(), "the code moved into the user's folder");
    // And from then on the user's folder alone answers.
    assert!(act::status_migrating(None, Path::new(&stored), &id).is_ok());
}
