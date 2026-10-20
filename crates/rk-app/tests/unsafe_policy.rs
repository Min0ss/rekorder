//! Enforces the workspace unsafe policy: every crate except `rk-win` must
//! declare `#![forbid(unsafe_code)]` at its crate root.
#![allow(clippy::panic)]

use std::fs;
use std::path::Path;

const FORBID: &str = "#![forbid(unsafe_code)]";

/// Crate roots that must forbid unsafe code, relative to `crates/`.
const SAFE_ROOTS: &[&str] = &[
    "rk-core/src/lib.rs",
    "rk-mux/src/lib.rs",
    "rk-app/src/main.rs",
];

/// The single crate allowed to contain `unsafe`.
const UNSAFE_CRATE: &str = "rk-win";

fn crates_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| panic!("rk-app has no parent directory"))
}

#[test]
fn safe_crates_forbid_unsafe_code() {
    for root in SAFE_ROOTS {
        let path = crates_dir().join(root);
        let src = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            src.contains(FORBID),
            "{} must contain {FORBID}",
            path.display()
        );
    }
}

#[test]
fn every_crate_is_covered_by_the_policy() {
    let entries = fs::read_dir(crates_dir()).unwrap_or_else(|e| panic!("cannot list crates: {e}"));
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("cannot read entry: {e}"));
        if !entry.path().join("Cargo.toml").is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let covered = name == UNSAFE_CRATE
            || SAFE_ROOTS
                .iter()
                .any(|r| r.starts_with(&format!("{name}/")));
        assert!(
            covered,
            "crate `{name}` is not listed in the unsafe policy test"
        );
    }
}
