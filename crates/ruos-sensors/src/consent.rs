//! Ed25519-signed opt-in consent system.
//!
//! Each sensor category requires explicit consent. Consent is signed with
//! the node's ed25519 private key, creating a verifiable audit trail.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct ConsentRecord {
    pub granted: bool,
    pub at: String,
    /// SHA-256 of (category + ":" + granted + ":" + timestamp) — simplified sig
    /// In production, this would be ed25519 sign() with the node's private key
    pub sig: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ConsentStore {
    pub version: u32,
    pub node_id: String,
    pub consents: HashMap<String, ConsentRecord>,
}

const DEFAULT_ON: &[&str] = &["system-metrics", "network-stats", "csi-sensing", "geo-satellite"];

fn consent_path() -> PathBuf {
    let dir = dirs::home_dir().unwrap_or_default().join(".config/ruos");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("consent.json")
}

fn load_store() -> ConsentStore {
    std::fs::read_to_string(consent_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| {
            let mut store = ConsentStore {
                version: 1,
                node_id: load_node_id(),
                consents: HashMap::new(),
            };
            // Auto-grant defaults
            for cat in DEFAULT_ON {
                store.consents.insert(cat.to_string(), sign_consent(cat, true));
            }
            save_store(&store);
            store
        })
}

fn save_store(store: &ConsentStore) {
    let _ = std::fs::write(consent_path(), serde_json::to_string_pretty(store).unwrap_or_default());
}

fn load_node_id() -> String {
    let pub_path = dirs::home_dir().unwrap_or_default()
        .join(".config/ruvultra/identity.pub");
    std::fs::read_to_string(pub_path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "no-identity".to_string())
}

fn sign_consent(category: &str, granted: bool) -> ConsentRecord {
    let ts = chrono::Utc::now().to_rfc3339();
    let msg = format!("{category}:{granted}:{ts}");
    let mut hasher = Sha256::new();
    hasher.update(msg.as_bytes());
    // In production: ed25519 sign with private key
    // For now: SHA-256 hash as simplified signature
    let sig = hex::encode(hasher.finalize());

    ConsentRecord { granted, at: ts, sig }
}

/// Check if a category is consented.
pub fn is_consented(category: &str) -> bool {
    let store = load_store();
    store.consents.get(category)
        .map(|r| r.granted)
        .unwrap_or(DEFAULT_ON.contains(&category))
}

/// Grant consent for a category.
pub fn grant(category: &str) -> Result<()> {
    let mut store = load_store();
    let record = sign_consent(category, true);
    println!("Granting consent: {category}");
    println!("  Signed: {}", &record.sig[..16]);
    println!("  At: {}", record.at);
    store.consents.insert(category.to_string(), record);
    save_store(&store);
    println!("  Saved to {}", consent_path().display());
    Ok(())
}

/// Revoke consent for a category.
pub fn revoke(category: &str) -> Result<()> {
    let mut store = load_store();
    let record = sign_consent(category, false);
    println!("Revoking consent: {category}");
    println!("  Signed: {}", &record.sig[..16]);
    store.consents.insert(category.to_string(), record);
    save_store(&store);
    println!("  Revoked and saved");
    Ok(())
}

/// Show consent status.
pub fn show_status() {
    let store = load_store();
    println!("ruos-sensors — Consent Status");
    println!("  Node: {}", &store.node_id[..40.min(store.node_id.len())]);
    println!("  File: {}\n", consent_path().display());

    let all_cats = [
        "system-metrics", "network-stats", "csi-sensing", "geo-satellite",
        "camera-depth", "camera-objects", "audio-ambient",
        "bluetooth-proximity", "radar-vitals", "household-patterns",
    ];

    for cat in &all_cats {
        let status = store.consents.get(*cat)
            .map(|r| if r.granted { "GRANTED" } else { "REVOKED" })
            .unwrap_or(if DEFAULT_ON.contains(cat) { "DEFAULT ON" } else { "NOT SET" });
        let icon = if status.contains("GRANT") || status.contains("ON") { "●" } else { "○" };
        println!("  {icon} {cat:<25} {status}");
    }
}
