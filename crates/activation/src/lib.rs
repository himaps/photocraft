//! photocraft-activation — the local activation marker.
//!
//! Not copy protection, and there is no online check: the code only marks that this
//! installation was registered with the author, who counts the machine codes that arrive and
//! answers each one with an activation code of the same length. The machine identifier never
//! leaves the machine unless the user sends the word code themselves, and the derivation below
//! ships inside the binary, so a determined user can compute an activation code without asking.
//! That is accepted by design (the requirement is a ledger, not a lock).
//!
//! The words exist so a code can be read out over a phone call: two-syllable Chinese words are
//! short and unambiguous where hex digits trip over pronunciation.
//!
//! Layout of a code: SHA-256 of the machine identifier (canonicalised) gives `digest`; every
//! code has [`CODE_LEN`] words; the machine code takes `digest[i] & 63` as the
//! word index of position `i`. The activation code is the same length and derives from the
//! machine code the user sent — `sha256(KEY ++ APP_ID ++ machine_indices)[i] & 63` — because the
//! author never sees the machine identifier, only its words.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::PathBuf;

/// The 64 activation words, by index. Given by the author; do not reorder.
pub const WORDS: [&str; 64] = [
    "富强", "民主", "文明", "和谐", "自由", "平等", "公正", "法治", "爱国", "敬业", "诚信", "友善", "昌隆", "安泰", "开化", "盛平", "兴邦", "民本", "崇文",
    "和睦", "自主", "均平", "公允", "遵法", "忠邦", "恪职", "立信", "亲睦", "弘道", "笃行", "崇信", "尚和", "守正", "公义", "平允", "依规", "严明", "循法",
    "明德", "持公", "普惠", "安民", "固本", "维新", "合众", "宽和", "明断", "守矩", "崇德", "厚善", "守诚", "敬责", "报国", "敦睦", "慎行", "守信", "向善",
    "弘毅", "克己", "怀仁", "崇礼", "持节", "谦恭", "淳良",
];

/// Separates the words of a code, in both directions.
pub const SEPARATOR: char = '-';

/// The response key. Embedded in the shipped binary, so it is an obfuscation, not a secret: it
/// only makes the activation code non-obvious to someone who has not read past this comment.
const KEY: &[u8] = b"PhotoCraft Kylin activation ledger key 1 (a marker, not a secret)";

#[derive(Debug, thiserror::Error)]
pub enum ActivationError {
    #[error("cannot read a hardware identifier on this machine (tried {0})")]
    NoMachineId(String),
    #[error("reading the hardware identifier: {0}")]
    Io(String),
    #[error("cannot write {0}: {1}")]
    Write(String, String),
    #[error("{word:?} is not one of the {total} activation words")]
    BadWord { word: String, total: usize },
    #[error("a code has {expected} words, this one has {got}")]
    WrongLength { expected: usize, got: usize },
    #[error("the activation code does not match this machine")]
    Mismatch,
    #[error("no activation is stored")]
    NotActivated,
}

/// A code: one word index per position, each in `0..64`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code {
    pub indices: Vec<u8>,
}

impl Code {
    /// The words joined with [`SEPARATOR`], in the form users read and type.
    pub fn render(&self) -> String {
        self.indices.iter().map(|&i| word(i)).collect::<Vec<_>>().join(&SEPARATOR.to_string())
    }

    /// The number of words, which both codes of one machine share.
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// The word at `index` (mod 64, so a raw hash byte is a valid index).
pub fn word(index: u8) -> &'static str {
    WORDS[index as usize % WORDS.len()]
}

/// The index of `word`, or [`ActivationError::BadWord`].
pub fn index(word: &str) -> Result<u8, ActivationError> {
    WORDS.iter().position(|w| *w == word).map(|i| i as u8).ok_or_else(|| ActivationError::BadWord { word: word.to_owned(), total: WORDS.len() })
}

/// Parses a typed or transmitted code: words separated by [`SEPARATOR`] (whitespace around a
/// word is kept out), each of them one of [`WORDS`].
pub fn parse(text: &str) -> Result<Code, ActivationError> {
    let indices = text.split(SEPARATOR).map(|w| w.trim()).filter(|w| !w.is_empty()).map(index).collect::<Result<Vec<_>, _>>()?;
    if indices.is_empty() {
        return Err(ActivationError::BadWord { word: text.trim().to_owned(), total: WORDS.len() });
    }
    Ok(Code { indices })
}

/// The machine identifier the codes are derived from. The name says CPU serial, but processor
/// serial numbers have been disabled since the Pentium III, so this is the first stable
/// hardware identifier the OS exposes: on Windows the installation's MachineGuid, on Linux the
/// DMI product UUID or board serial, falling back to `/etc/machine-id` (per installation, and
/// what the activation actually marks: this installation on this machine).
pub fn machine_id() -> Result<String, ActivationError> {
    machine_id_platform().map(|id| id.trim().to_lowercase())
}

#[cfg(target_os = "linux")]
fn machine_id_platform() -> Result<String, ActivationError> {
    const FILES: [&str; 4] = ["/sys/class/dmi/id/product_uuid", "/sys/class/dmi/id/board_serial", "/etc/machine-id", "/var/lib/dbus/machine-id"];
    let found = FILES.iter().find_map(|f| std::fs::read_to_string(f).ok()).filter(|s| !s.trim().is_empty());
    found.ok_or_else(|| ActivationError::NoMachineId(FILES.join(", ")))
}

#[cfg(target_os = "windows")]
fn machine_id_platform() -> Result<String, ActivationError> {
    use winsafe::{HKEY, RegistryValue, co::RRF};
    match HKEY::LOCAL_MACHINE.RegGetValue(Some(r"SOFTWARE\Microsoft\Cryptography"), Some("MachineGuid"), RRF::RT_REG_SZ) {
        Ok(RegistryValue::Sz(v)) if !v.trim().is_empty() => Ok(v),
        _ => Err(ActivationError::NoMachineId(r"HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid".to_owned())),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn machine_id_platform() -> Result<String, ActivationError> {
    Err(ActivationError::NoMachineId("this platform".to_owned()))
}

/// The digest both codes derive from: the canonicalised machine identifier with a domain tag.
fn digest(machine_id: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(machine_id.trim().to_lowercase());
    h.update(b"\nphotocraft-activation-v1");
    h.finalize().to_vec()
}

/// Every code has this many words: the machine code and the activation code of one machine
/// share the fixed length.
pub const CODE_LEN: usize = 4;

/// The code this machine shows to its user, to be sent to the author.
pub fn machine_code(machine_id: &str) -> Code {
    let d = digest(machine_id);
    Code { indices: (0..CODE_LEN).map(|i| d[i] & 63).collect() }
}

/// The code the author answers with: same length as `machine`, derived from the machine code
/// the user sent through the response key and the [`APP_ID`] — so the author, who never sees
/// the machine identifier, can compute it from the words alone; and a code computed for one
/// app cannot activate another.
pub fn activation_response(machine: &Code) -> Code {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(KEY);
    h.update(APP_ID.as_bytes());
    h.update(&machine.indices);
    let r = h.finalize();
    Code { indices: (0..machine.len()).map(|i| r[i] & 63).collect() }
}

/// The code the author answers with, for this machine.
pub fn activation_code(machine_id: &str) -> Code {
    activation_response(&machine_code(machine_id))
}

/// The software identifier, so an activation code computed for one app cannot activate
/// another: the APP_ID is mixed into the response hash alongside the machine code indices.
pub const APP_ID: &str = "photocraft";

/// The activation file's name, in every location it has had.
const LICENSE_FILE: &str = ".license.md";

/// The folder in the user's home directory that holds the activation.
const DATA_DIR: &str = ".photocraft";

/// The user's home directory for the activation: `%USERPROFILE%` on Windows, `$HOME`
/// elsewhere. `env` reads a variable, so the rules are testable without touching the process
/// environment. `None` when the platform reports none.
fn home_dir(env: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    let key = if cfg!(target_os = "windows") { "USERPROFILE" } else { "HOME" };
    env(key).filter(|d| !d.is_empty()).map(PathBuf::from)
}

/// [`data_dir`] for the home directory `env` reports (see [`home_dir`]).
pub fn data_dir_in(env: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    home_dir(env).map(|home| home.join(DATA_DIR))
}

/// [`storage_path`] for the home directory `env` reports (see [`home_dir`]).
pub fn storage_path_in(env: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    data_dir_in(env).map(|dir| dir.join(LICENSE_FILE))
}

/// The directory the activation lives in: `~/.photocraft`, the user's own folder, so storing
/// the code needs no administrator rights (the install directory is root- or admin-owned on
/// every packaged platform). The same idea as ZCode's `~/.zcode/.license.md`.
/// `None` when the platform reports no home directory.
pub fn data_dir() -> Option<PathBuf> {
    data_dir_in(|key| std::env::var_os(key))
}

/// Where the activation is stored: `~/.photocraft/.license.md` (see [`data_dir`]). `None`
/// when the platform reports no home directory.
pub fn storage_path() -> Option<PathBuf> {
    storage_path_in(|key| std::env::var_os(key))
}

/// Where earlier versions stored the activation: a hidden Markdown file in the install
/// directory — `<exe>/../share/photocraft/.license.md` on Linux (FHS `bin/` → `share/`),
/// `<exe>/.license.md` on Windows. Read only to migrate (see [`status`]).
fn legacy_storage_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(if cfg!(target_os = "linux") { dir.join("../share/photocraft").join(LICENSE_FILE) } else { dir.join(LICENSE_FILE) })
}

/// Whether this machine is activated by the file `legacy` an earlier version stored beside the
/// installed files, moving it to `target` first (see [`migrate_between`]); then by `target`
/// itself. What [`status`] does, for two explicit paths.
pub fn status_migrating(legacy: Option<&std::path::Path>, target: &std::path::Path, machine_id: &str) -> Result<Code, ActivationError> {
    if let Some(legacy) = legacy {
        migrate_between(legacy, target, machine_id);
    }
    status_at(&target.to_path_buf(), machine_id)
}

/// Moves the activation in `legacy` to `target`, when `legacy` holds this app's valid code for
/// this machine and `target` is free: anything else is left alone (the install directory may
/// not be writable). A failure to move is not an error: the caller then reports the machine as
/// not activated, and the next start tries again.
fn migrate_between(legacy: &std::path::Path, target: &std::path::Path, machine_id: &str) {
    if legacy == target || target.exists() || !legacy.is_file() {
        return;
    }
    let Ok(Ok(stored)) = std::fs::read_to_string(legacy).map(|content| status_content(&content, machine_id)) else { return };
    let Some(dir) = target.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    if std::fs::write(target, license_file_content(&stored)).is_ok() {
        // The copy is in place; the old file is this installation's and may go (best effort —
        // it usually is not writable, and then it simply stays).
        let _ = std::fs::remove_file(legacy);
    }
}

/// The `.license.md` text for `code`: the software ID on the first line, the code on the second.
fn license_file_content(code: &Code) -> String {
    format!("# {}\n- 激活码: {}\n", APP_ID, code.render())
}

/// Whether the `.license.md` text `content` holds the activation code of this machine. The
/// file has the software ID on its first line and the activation code on its second; the
/// software ID must match [`APP_ID`] so a license from one app cannot activate another.
fn status_content(content: &str, machine_id: &str) -> Result<Code, ActivationError> {
    let mut lines = content.lines();
    let software = lines.next().ok_or(ActivationError::NotActivated)?.strip_prefix("# ").ok_or(ActivationError::Mismatch)?;
    if software != APP_ID {
        return Err(ActivationError::Mismatch);
    }
    let code_line = lines.next().ok_or(ActivationError::NotActivated)?;
    let stored = parse(code_line.strip_prefix("- 激活码: ").unwrap_or(code_line.trim()))?;
    let expected = activation_code(machine_id);
    if stored.len() != expected.len() {
        return Err(ActivationError::WrongLength { expected: expected.len(), got: stored.len() });
    }
    if stored.indices != expected.indices {
        return Err(ActivationError::Mismatch);
    }
    Ok(stored)
}

/// Whether `path` holds the activation code of this machine (see [`status_content`]).
pub fn status_at(path: &PathBuf, machine_id: &str) -> Result<Code, ActivationError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { ActivationError::NotActivated } else { ActivationError::Io(e.to_string()) })?;
    status_content(&content, machine_id)
}

/// Whether this machine is activated: an activation stored in [`storage_path`], or one an
/// earlier version stored beside the installed files, which is moved there first.
pub fn status(machine_id: &str) -> Result<Code, ActivationError> {
    match storage_path() {
        Some(target) => status_migrating(legacy_storage_path().as_deref(), &target, machine_id),
        None => Err(ActivationError::NoMachineId("no home directory".to_owned())),
    }
}

/// Verifies the typed activation code against this machine and stores it as a `.license.md`
/// recording the activation details. `at` is [`storage_path`]; passing it in keeps this testable.
/// The directory is created when it does not exist yet.
pub fn activate_at(at: &PathBuf, machine_id: &str, typed: &str) -> Result<Code, ActivationError> {
    let typed = parse(typed)?;
    let expected = activation_code(machine_id);
    if typed.len() != expected.len() {
        return Err(ActivationError::WrongLength { expected: expected.len(), got: typed.len() });
    }
    if typed.indices != expected.indices {
        return Err(ActivationError::Mismatch);
    }
    if let Some(dir) = at.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        return Err(ActivationError::Write(dir.display().to_string(), e.to_string()));
    }
    std::fs::write(at, license_file_content(&typed)).map_err(|e| ActivationError::Write(at.display().to_string(), e.to_string()))?;
    Ok(typed)
}

/// Verifies and stores the activation (see [`storage_path`]).
pub fn activate(machine_id: &str, typed: &str) -> Result<Code, ActivationError> {
    match storage_path() {
        Some(p) => activate_at(&p, machine_id, typed),
        None => Err(ActivationError::NoMachineId("no home directory".to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_word_table_is_the_given_one() {
        assert_eq!(WORDS.len(), 64);
        assert_eq!(WORDS[0], "富强");
        assert_eq!(WORDS[63], "淳良");
        assert_eq!(WORDS.iter().collect::<std::collections::BTreeSet<_>>().len(), 64, "the words are distinct");
        assert!(WORDS.iter().all(|w| w.chars().count() == 2), "two characters each, so codes read evenly");
    }

    #[test]
    fn codes_render_and_parse_round_trip() {
        let code = Code { indices: vec![0, 26, 63, 7] };
        let text = code.render();
        assert_eq!(text, "富强-立信-淳良-法治");
        assert_eq!(parse(&text).unwrap(), code);
        // Whitespace around a word is typing, not part of it.
        assert_eq!(parse(" 富强 - 立信 - 淳良 - 法治 ").unwrap(), code);
    }

    #[test]
    fn unknown_words_and_empty_input_are_rejected() {
        assert!(matches!(parse("富强-民主-文明-和谐-X"), Err(ActivationError::BadWord { .. })));
        assert!(matches!(parse("hello"), Err(ActivationError::BadWord { .. })));
        assert!(matches!(parse("   "), Err(ActivationError::BadWord { .. })));
        assert!(matches!(parse("-"), Err(ActivationError::BadWord { .. })));
    }

    /// Vectors produced by the same derivation in PowerShell (.NET SHA256), so a future
    /// refactor of the hashing is a deliberate change, not an accident. The activation code
    /// derives from the machine *words* (what the author receives), not from the machine id.
    #[test]
    fn known_machine_ids_have_known_codes() {
        let cases: [(&str, usize, &[u8], &[u8]); 3] = [
            ("test-machine-001", 4, &[43, 39, 2, 13], &[2, 23, 34, 34]),
            ("KYLIN-V20-Desktop", 4, &[50, 55, 45, 24], &[45, 52, 61, 55]),
            ("linux-abc123", 4, &[12, 20, 34, 40], &[46, 2, 59, 50]),
        ];
        for (id, n, user, act) in cases {
            let machine = machine_code(id);
            assert_eq!(machine.len(), n, "{id}");
            assert_eq!(machine.indices, user, "{id}");
            // From the identifier (the app's view)…
            let activation = activation_code(id);
            assert_eq!(activation.len(), n, "{id}");
            assert_eq!(activation.indices, act, "{id}");
            // …and from the words alone (the author's view): both agree.
            assert_eq!(activation_response(&parse(&machine.render()).unwrap()), activation, "{id}");
            assert_eq!(parse(&activation.render()).unwrap(), activation, "{id}");
        }
    }

    /// Both codes of one machine always have the same fixed length ([`CODE_LEN`]).
    #[test]
    fn code_lengths_are_four_and_agree() {
        for n in 0..200u64 {
            let id = format!("machine-{n}");
            let machine = machine_code(&id);
            let activation = activation_code(&id);
            assert_eq!(machine.len(), activation.len());
            assert_eq!(machine.len(), CODE_LEN, "{id}");
        }
    }

    #[test]
    fn activation_verifies_and_a_wrong_code_does_not() {
        let dir = std::env::temp_dir().join(format!("photocraft-activation-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let at = dir.join(".license.md");
        let id = "test-machine-001";
        assert!(matches!(status_at(&at, id), Err(ActivationError::NotActivated)));
        // The right activation code (typed as a user would: with stray spaces) is accepted and
        // stored.
        let typed = activation_code(id).render();
        activate_at(&at, id, &format!("  {typed}  ")).unwrap();
        assert!(status_at(&at, id).is_ok());
        // A wrong code of the right length (this machine's response with its first word
        // changed to a different one) is rejected as a mismatch — deterministic, unlike a
        // code derived from another machine whose word count could differ.
        let mut wrong = activation_code(id).indices;
        wrong[0] = (wrong[0] + 1) % 64;
        let wrong = Code { indices: wrong }.render();
        assert!(matches!(activate_at(&at, id, &wrong), Err(ActivationError::Mismatch)));
        // A code of a different length is a length error before it is a mismatch.
        let long = format!("{typed}-{typed}");
        assert!(matches!(activate_at(&at, id, &long), Err(ActivationError::WrongLength { expected: 4, got: 8 })));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The activation lives in the user's own folder (`~/.photocraft/.license.md`), never
    /// beside the program: the installed files are admin- or root-owned, the home directory is
    /// the user's. With no home directory there is nowhere to store it.
    #[test]
    fn storage_is_in_the_home_directory_not_beside_the_program() {
        let env = |key: &str| (key == if cfg!(target_os = "windows") { "USERPROFILE" } else { "HOME" }).then(|| "C:/Users/ann".into());
        let stored = storage_path_in(env).unwrap();
        assert!(stored.starts_with(PathBuf::from("C:/Users/ann")), "{} is not under the home directory", stored.display());
        assert!(stored.to_string_lossy().replace('\\', "/").ends_with("/.photocraft/.license.md"), "{stored:?}");
        assert!(!stored.to_string_lossy().replace('\\', "/").contains("share/photocraft"), "not the old install-directory location: {stored:?}");
        assert!(storage_path_in(|_| None).is_none(), "no home directory, nowhere to store");
    }

    /// An activation an earlier version stored beside the installed files still counts after
    /// the move to the home directory, so an activated machine is not asked again.
    #[test]
    fn an_activation_beside_the_program_is_moved_to_the_home_directory() {
        let id = "test-machine-001";
        // Both under the test's own temp folder; nothing touches the real home directory.
        let root = std::env::temp_dir().join(format!("photocraft-activation-move-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let legacy = root.join("usr").join("share").join("photocraft").join(LICENSE_FILE);
        let target = root.join("home").join(".photocraft").join(LICENSE_FILE);
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let ask = || status_migrating(Some(legacy.as_path()), &target, id);

        assert!(matches!(ask(), Err(ActivationError::NotActivated)), "nothing stored yet");
        assert!(!target.exists(), "no legacy file: no directory is created");

        std::fs::write(&legacy, license_file_content(&activation_code(id))).unwrap();
        assert!(ask().is_ok(), "the old file still activates");
        assert!(target.is_file() && !legacy.exists(), "the code moved to the home directory");

        // A file of another app, or with a code for another machine, never counts and is left alone.
        let _ = std::fs::remove_dir_all(target.parent().unwrap());
        std::fs::write(&legacy, "# other-app\n- 激活码: 富强-民主-文明-和谐\n").unwrap();
        assert!(ask().is_err() && legacy.exists(), "another app's file is left where it is");
        std::fs::write(&legacy, format!("# {APP_ID}\n- 激活码: 富强-民主-文明-和谐\n")).unwrap();
        assert!(ask().is_err() && legacy.exists(), "a code for another machine is left where it is");

        // An activation already in the home directory wins: the old file is not touched.
        std::fs::write(&legacy, license_file_content(&activation_code(id))).unwrap();
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, license_file_content(&activation_code(id))).unwrap();
        assert!(ask().is_ok() && legacy.exists(), "the old file is left alone once the home directory has one");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Storing creates the user's directory when it does not exist yet (a fresh profile).
    #[test]
    fn storing_creates_the_directory() {
        let root = std::env::temp_dir().join(format!("photocraft-activation-fresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let at = root.join(".photocraft").join(LICENSE_FILE);
        let id = "test-machine-001";
        activate_at(&at, id, &activation_code(id).render()).unwrap();
        assert!(status_at(&at, id).is_ok());
        assert!(std::fs::read_to_string(&at).unwrap().starts_with("# photocraft\n- 激活码: "), "the file format is unchanged");
        let _ = std::fs::remove_dir_all(&root);
    }
}
