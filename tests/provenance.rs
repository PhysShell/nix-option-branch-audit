//! Fixture provenance lock: proves each vendored (non-mutation,
//! non-synthetic) fixture file is still byte-for-byte what
//! `fixtures/provenance.toml` claims it is -- pulled from a specific
//! upstream commit, not just a comment asserting so. A fixture "cleaned
//! up for convenience" without updating the lock fails loudly here,
//! instead of the test suite staying green while "real historical commit"
//! quietly becomes fiction.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
struct ProvenanceFile {
    fixture: Vec<FixtureEntry>,
}

#[derive(Deserialize)]
struct FixtureEntry {
    path: String,
    repo: String,
    commit: String,
    upstream_path: String,
    sha256: String,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn vendored_fixtures_match_locked_provenance() {
    let lock_path = manifest_dir().join("fixtures/provenance.toml");
    let lock: ProvenanceFile = toml::from_str(
        &fs::read_to_string(&lock_path).expect("fixtures/provenance.toml must exist"),
    )
    .expect("fixtures/provenance.toml must parse");

    assert!(
        !lock.fixture.is_empty(),
        "provenance lock must not be empty"
    );

    let mut mismatches = Vec::new();
    for entry in &lock.fixture {
        let full_path = manifest_dir().join(&entry.path);
        let contents = fs::read(&full_path)
            .unwrap_or_else(|e| panic!("locked fixture {} does not exist: {e}", entry.path));
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let actual = format!("{:x}", hasher.finalize());
        if actual != entry.sha256 {
            mismatches.push(format!(
                "{}: locked sha256={} actual={} (claimed source: {}@{}:{})",
                entry.path, entry.sha256, actual, entry.repo, entry.commit, entry.upstream_path
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "fixture(s) no longer match their locked provenance -- if this is a \
         deliberate update, recompute with `sha256sum` and update \
         fixtures/provenance.toml; if not, something modified a vendored \
         fixture silently:\n{}",
        mismatches.join("\n")
    );
}
