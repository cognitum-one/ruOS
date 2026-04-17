//! Self-monitor: check ruOS services, restart if crashed, GPU thermal.

use crate::{http_get_json, log, BRAIN_URL, EMBEDDER_URL};
use std::process::Command;

fn is_service_active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", unit])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false)
}

fn restart_service(unit: &str) {
    let _ = Command::new("systemctl")
        .args(["--user", "restart", unit])
        .output();
}

pub async fn self_monitor() -> Vec<String> {
    let mut issues = Vec::new();

    let services = [
        ("ruvultra-brain.service", format!("{BRAIN_URL}/health")),
        ("ruvultra-embedder.service", format!("{EMBEDDER_URL}/health")),
    ];

    for (unit, health_url) in &services {
        let active = is_service_active(unit);
        let healthy = if active { http_get_json(health_url).await.is_some() } else { false };

        if !active {
            log("WARN", &format!("RESTART: {unit} not active, restarting"));
            restart_service(unit);
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if is_service_active(unit) {
                log("INFO", &format!("RECOVERED: {unit}"));
            } else {
                log("ERROR", &format!("FAILED: {unit} unrecoverable"));
                issues.push(format!("{unit} down"));
            }
        } else if !healthy {
            log("WARN", &format!("UNHEALTHY: {unit}, restarting"));
            restart_service(unit);
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            issues.push(format!("{unit} was unhealthy"));
        }
    }

    // GPU thermal check
    if let Some(temp) = gpu_temp() {
        if temp > 85 {
            log("WARN", &format!("GPU thermal: {temp}C"));
            issues.push(format!("GPU {temp}C"));
        }
    }

    issues
}

fn gpu_temp() -> Option<u32> {
    Command::new("nvidia-smi")
        .args(["--query-gpu=temperature.gpu", "--format=csv,noheader,nounits"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
}
