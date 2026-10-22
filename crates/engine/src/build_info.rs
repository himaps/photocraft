//! Version and build provenance shared by every frontend (About dialog, `--version`).
//!
//! The version is the workspace version (`[workspace.package] version` in the root
//! `Cargo.toml`, managed by `cargo xtask version`). The commit and build date are optional and
//! come from environment variables at compile time, set by CI and the packaging scripts:
//!
//! - `PHOTOCRAFT_BUILD_SHA`: the git commit (any length; shown shortened to 9 characters)
//! - `PHOTOCRAFT_BUILD_DATE`: the build date, conventionally `YYYY-MM-DD`
//! - `PHOTOCRAFT_BUILD_LABEL`: a short name for a non-standard build of the same version
//!   (`kylin` for the glibc-2.31 baseline build), shown right after the version so the two
//!   can't be mistaken for each other
//!
//! Without them (a plain `cargo build`) the build reports itself as a dev build. Nothing here
//! shells out to git, so builds work from a source tarball and for wasm.

/// The workspace version, such as `0.1.0` or `0.2.0-rc.1`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The full git commit this build was made from, when the build environment recorded it.
pub fn git_sha() -> Option<&'static str> {
    option_env!("PHOTOCRAFT_BUILD_SHA").map(str::trim).filter(|s| !s.is_empty())
}

/// The build date (`YYYY-MM-DD`), when the build environment recorded it.
pub fn build_date() -> Option<&'static str> {
    option_env!("PHOTOCRAFT_BUILD_DATE").map(str::trim).filter(|s| !s.is_empty())
}

/// The compile-time build label (`kylin` for the Kylin glibc-2.31 baseline build), when set.
pub const LABEL: Option<&str> = option_env!("PHOTOCRAFT_BUILD_LABEL");

/// The commit shortened for display.
pub fn short_sha() -> Option<&'static str> {
    git_sha().map(|s| &s[..s.len().min(9)])
}

/// `0.1.0 (3f2a9c1d0, 2026-10-01)`, `0.1.0 (kylin; 3f2a9c1d0, 2026-10-01)` with a label, or
/// `0.1.0 (dev build)` without provenance.
pub fn long_version() -> String {
    format_long(VERSION, LABEL, short_sha(), build_date())
}

/// Assemble the display version: a labelled build puts the label first inside the parentheses
/// and any provenance after a semicolon; the release formats keep commit and date inside one
/// pair of parentheses. A blank label is the same as none.
fn format_long(version: &str, label: Option<&str>, sha: Option<&str>, date: Option<&str>) -> String {
    let label = label.map(str::trim).filter(|l| !l.is_empty());
    match (label, sha, date) {
        (Some(l), Some(s), Some(d)) => format!("{version} ({l}; {s}, {d})"),
        (Some(l), Some(s), None) => format!("{version} ({l}; {s})"),
        (Some(l), None, Some(d)) => format!("{version} ({l}; {d})"),
        (Some(l), None, None) => format!("{version} ({l})"),
        (None, Some(s), Some(d)) => format!("{version} ({s}, {d})"),
        (None, Some(s), None) => format!("{version} ({s})"),
        (None, None, Some(d)) => format!("{version} ({d})"),
        (None, None, None) => format!("{version} (dev build)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_version_formats() {
        assert_eq!(format_long("1.2.3", None, Some("abc"), Some("2026-10-01")), "1.2.3 (abc, 2026-10-01)");
        assert_eq!(format_long("1.2.3", None, Some("abc"), None), "1.2.3 (abc)");
        assert_eq!(format_long("1.2.3", None, None, Some("2026-10-01")), "1.2.3 (2026-10-01)");
        assert_eq!(format_long("1.2.3-rc.1", None, None, None), "1.2.3-rc.1 (dev build)");
    }

    #[test]
    fn a_label_follows_the_version_and_a_blank_one_is_ignored() {
        assert_eq!(format_long("1.2.3", Some("kylin"), Some("abc"), Some("2026-10-01")), "1.2.3 (kylin; abc, 2026-10-01)");
        assert_eq!(format_long("1.2.3", Some("kylin"), None, None), "1.2.3 (kylin)");
        assert_eq!(format_long("1.2.3", Some("kylin"), Some("abc"), None), "1.2.3 (kylin; abc)");
        // A blank or whitespace label is the same as none.
        assert_eq!(format_long("1.2.3", Some(""), Some("abc"), None), "1.2.3 (abc)");
        assert_eq!(format_long("1.2.3", Some("  "), None, None), "1.2.3 (dev build)");
    }

    #[test]
    fn version_is_the_workspace_version() {
        assert!(!VERSION.is_empty());
        assert!(long_version().starts_with(VERSION));
        if let Some(s) = short_sha() {
            assert!(s.len() <= 9);
        }
    }
}
