//! OTA updates: check GitHub releases, download + install.

use crate::{log, read_state, write_state};
use chrono::Local;
use serde_json::json;
use std::process::Command;

const REPO: &str = "cognitum-one/ruOS";

fn installed_version() -> String {
    read_state("installed_version")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn latest_release() -> Option<String> {
    let output = Command::new("gh")
        .args(["release", "view", "--repo", REPO, "--json", "tagName", "-q", ".tagName"])
        .output().ok()?;
    let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if tag.is_empty() { None } else { Some(tag) }
}

pub async fn update(check_only: bool, force: bool) {
    log("INFO", "=== ruos-update check ===");

    let current = installed_version();
    let latest = match latest_release() {
        Some(v) => v,
        None => {
            log("ERROR", "Could not determine latest release");
            return;
        }
    };

    log("INFO", &format!("  Current: {current}  Latest: {latest}"));

    if current == latest && !force {
        println!("Up to date: {current}");
        return;
    }

    if check_only {
        if current != latest {
            println!("Update available: {current} → {latest}");
        } else {
            println!("Up to date: {current}");
        }
        return;
    }

    log("INFO", &format!("UPDATE: {current} → {latest}"));

    // Download .debs
    let tmpdir = std::env::temp_dir().join(format!("ruos-update-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmpdir);

    let arch = if cfg!(target_arch = "x86_64") { "amd64" }
        else if cfg!(target_arch = "aarch64") { "arm64" }
        else { "amd64" };

    let patterns = [
        format!("ruos-core_*_{arch}.deb"),
        "ruos-brain-base_*.deb".to_string(),
        "ruos-agent_*.deb".to_string(),
    ];

    for pattern in &patterns {
        let status = Command::new("gh")
            .args(["release", "download", &latest, "--repo", REPO,
                   "--pattern", pattern, "--dir", tmpdir.to_str().unwrap_or(".")])
            .output();
        if let Ok(o) = status {
            if o.status.success() {
                log("INFO", &format!("  Downloaded: {pattern}"));
            }
        }
    }

    // Count downloads
    let debs: Vec<_> = std::fs::read_dir(&tmpdir)
        .into_iter()
        .flat_map(|d| d.filter_map(|e| e.ok()).filter(|e| {
            e.path().extension().map(|ext| ext == "deb").unwrap_or(false)
        }))
        .collect();

    if debs.is_empty() {
        log("ERROR", "No .deb files downloaded");
        let _ = std::fs::remove_dir_all(&tmpdir);
        return;
    }

    // Stop services
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "ruvultra-brain.service", "ruvultra-embedder.service"])
        .output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Install
    for deb in &debs {
        let _ = Command::new("sudo")
            .args(["dpkg", "-i", deb.path().to_str().unwrap_or("")])
            .output();
    }

    // Restart
    let _ = Command::new("systemctl")
        .args(["--user", "start", "ruvultra-brain.service", "ruvultra-embedder.service"])
        .output();

    write_state("installed_version", &json!(latest));
    write_state("last_update", &json!({
        "at": Local::now().to_rfc3339(),
        "from": current,
        "to": latest,
        "packages": debs.len(),
    }));

    log("INFO", &format!("UPDATE: installed {latest} ({} packages)", debs.len()));

    let _ = std::fs::remove_dir_all(&tmpdir);
}
