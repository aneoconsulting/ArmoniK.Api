//! `ffi/schema/generated/manifest.json` as the reference every arm is checked against.
//!
//! It is VALIDATED (stage 1: prost 0.14.4 and prost-reflect 0.16.5 agree with it on every
//! payload but P7.1, which no canonical writer can produce). A slice that disagrees with a
//! hash now has a defect in itself, so this is the oracle rather than a second opinion.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct Row {
    pub bytes: usize,
    pub sha256: String,
    /// Present only for the payloads at or under 64 KB, which are committed as vectors.
    pub vector: Option<Vec<u8>>,
}

pub struct Manifest(pub BTreeMap<String, Row>);

pub fn generated_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../schema/generated")
        .canonicalize()
        .expect("ffi/schema/generated is not where this crate expects it")
}

impl Manifest {
    pub fn load() -> Self {
        let dir = generated_dir();
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
        assert!(
            v["status"].as_str().unwrap().starts_with("VALIDATED"),
            "the manifest is not validated; run the stage 1 harness before trusting it"
        );
        let mut out = BTreeMap::new();
        for (pid, row) in v["payloads"].as_object().unwrap() {
            out.insert(
                pid.clone(),
                Row {
                    bytes: row["bytes"].as_u64().unwrap() as usize,
                    sha256: row["sha256"].as_str().unwrap().to_string(),
                    vector: row
                        .get("vector")
                        .and_then(|p| p.as_str())
                        .map(|p| std::fs::read(dir.join(p)).unwrap()),
                },
            );
        }
        Manifest(out)
    }

    pub fn row(&self, pid: &str) -> &Row {
        self.0.get(pid).unwrap_or_else(|| panic!("no manifest row for {pid}"))
    }
}

pub fn sha(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|c| format!("{c:02x}")).collect()
}
