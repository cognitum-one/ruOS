//! Auto-profile: detect workload → switch GPU profile.
//! Uses LLM reasoning when available, falls back to rules.

use crate::{aidefence, http_post_json, log, read_state, write_state, LLM_URL, BRAIN_URL};
use chrono::Local;
use serde_json::{json, Value};
use std::process::Command;

pub async fn gpu_util() -> Option<u32> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=utilization.gpu,temperature.gpu,power.draw",
               "--format=csv,noheader,nounits"])
        .output().ok()?;
    let s = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = s.trim().split(',').map(|s| s.trim()).collect();
    parts.first()?.parse().ok()
}

struct GpuState {
    util: u32,
    temp: u32,
    power: f32,
    procs: Vec<String>,
}

fn read_gpu() -> Option<GpuState> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=utilization.gpu,temperature.gpu,power.draw",
               "--format=csv,noheader,nounits"])
        .output().ok()?;
    let s = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = s.trim().split(',').map(|s| s.trim()).collect();
    if parts.len() < 3 { return None; }

    let procs_out = Command::new("nvidia-smi")
        .args(["--query-compute-apps=process_name", "--format=csv,noheader"])
        .output().ok()?;
    let procs: Vec<String> = String::from_utf8_lossy(&procs_out.stdout)
        .lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();

    Some(GpuState {
        util: parts[0].parse().unwrap_or(0),
        temp: parts[1].parse().unwrap_or(0),
        power: parts[2].parse().unwrap_or(0.0),
        procs,
    })
}

fn current_profile() -> String {
    // Read from agent state first
    if let Some(v) = read_state("last_profile_switch") {
        if let Some(to) = v.get("to").and_then(|v| v.as_str()) {
            return to.to_string();
        }
    }
    // Fallback: read history
    let history = dirs::home_dir().unwrap_or_default()
        .join(".local/state/ruvultra-profiles/history.jsonl");
    if let Ok(content) = std::fs::read_to_string(&history) {
        for line in content.lines().rev() {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if v.get("action").and_then(|a| a.as_str()) == Some("apply-success") {
                    if let Some(name) = v.get("profile_name").and_then(|n| n.as_str()) {
                        if name != "__rollback__" {
                            return name.to_string();
                        }
                    }
                }
            }
        }
    }
    "cognitum-balanced".to_string()
}

fn apply_profile(name: &str) {
    let bin = dirs::home_dir().unwrap_or_default().join(".local/bin/ruvultra-profile");
    let _ = Command::new(bin).args(["apply", name]).output();
}

pub async fn auto_profile() -> Option<Value> {
    let gpu = read_gpu()?;
    let hour = Local::now().hour();
    let cur = current_profile();
    let mut recommended: Option<String> = None;
    let mut source = "rules";

    // Safety: thermal override (no LLM)
    if gpu.temp > 85 {
        recommended = Some("cognitum-idle".into());
        source = "safety-thermal";
    } else {
        // Try LLM reasoning (rate-limited)
        let should_reason = match read_state("last_reasoning") {
            Some(v) => {
                let last_util: i64 = v.get("observations")
                    .and_then(|o| o.get("gpu_util"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(-1);
                let state_changed = (gpu.util as i64 - last_util).unsigned_abs() > 20 || gpu.temp > 75;
                if state_changed { true } else {
                    let last_at = v.get("at").and_then(|v| v.as_str()).unwrap_or("");
                    chrono::DateTime::parse_from_rfc3339(last_at)
                        .map(|dt| Local::now().signed_duration_since(dt).num_minutes() > 15)
                        .unwrap_or(true)
                }
            }
            None => true,
        };

        if should_reason {
            if let Some(decision) = llm_reason(&gpu, &cur).await {
                let action = decision.get("action").and_then(|v| v.as_str()).unwrap_or("");
                if action == "switch_profile" {
                    if let Some(name) = decision.get("params")
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        let valid = ["gpu-train", "gpu-inference", "cognitum-balanced", "cognitum-idle"];
                        if valid.contains(&name) {
                            recommended = Some(name.to_string());
                            let reason = decision.get("reasoning").and_then(|v| v.as_str()).unwrap_or("");
                            source = "ruvllm";
                            log("INFO", &format!("RUVLLM: → {name} ({reason})"));
                        }
                    }
                }
            }
        }

        // Fallback rules
        if recommended.is_none() {
            if gpu.procs.iter().any(|p| p.contains("python") || p.contains("train")) {
                recommended = Some("gpu-train".into());
            } else if gpu.util > 80 {
                recommended = Some("gpu-inference".into());
            } else if gpu.util < 5 && gpu.temp < 50 && (1..=6).contains(&hour) {
                recommended = Some("cognitum-idle".into());
            } else if gpu.temp > 80 {
                recommended = Some("cognitum-idle".into());
            }
        }
    }

    if let Some(ref rec) = recommended {
        if rec != &cur {
            log("INFO", &format!("AUTO-PROFILE: {cur} → {rec} (gpu={}%, temp={}C, source={source})",
                gpu.util, gpu.temp));
            apply_profile(rec);
            write_state("last_profile_switch", &json!({
                "from": cur, "to": rec,
                "reason": format!("gpu={}% temp={}C hour={hour}", gpu.util, gpu.temp),
                "source": source,
                "at": Local::now().to_rfc3339(),
            }));
        }
    }

    Some(json!({
        "gpu_util": gpu.util, "gpu_temp": gpu.temp,
        "profile": cur, "recommended": recommended,
        "reasoning_source": source,
    }))
}

async fn llm_reason(gpu: &GpuState, current: &str) -> Option<Value> {
    // Check LLM is up
    crate::http_get_json(&format!("{LLM_URL}/v1/models")).await?;

    // RAG context from brain
    let rag_query = if gpu.util > 50 { "GPU workload optimization" }
        else { "system idle optimization power management" };
    let rag_resp = http_post_json(
        &format!("{BRAIN_URL}/brain/search"),
        &json!({"query": rag_query, "k": 3}),
    ).await.unwrap_or(json!({}));
    let rag_results = rag_resp.get("results").and_then(|v| v.as_array());
    let mut rag_text = String::new();
    if let Some(results) = rag_results {
        for r in results {
            let content = r.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if content.is_empty() { continue; }
            // AIDefence: scan RAG content
            if !aidefence::is_safe(content) {
                log("WARN", "AIDEFENCE: blocked RAG content");
                continue;
            }
            rag_text.push_str(&format!("- {}\n", &content[..200.min(content.len())]));
        }
    }

    let system_prompt = "You are ruos-agent. Given system state, decide an action. \
        Respond with ONLY JSON: {\"action\": \"...\", \"params\": {...}, \"reasoning\": \"...\"}\n\
        Actions: switch_profile(name), no_action";

    let user_prompt = format!(
        "GPU: {}% util, {}C, procs: {}\nProfile: {current}\nTime: {}\n\
         Brain context:\n{}\nDecide.",
        gpu.util, gpu.temp,
        if gpu.procs.is_empty() { "none".to_string() } else { gpu.procs.join(", ") },
        Local::now().format("%H:%M %A"),
        if rag_text.is_empty() { "none".to_string() } else { rag_text },
    );

    let body = json!({
        "model": "Qwen/Qwen2.5-3B-Instruct",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
        "temperature": 0.1,
        "max_tokens": 200,
    });

    let resp = http_post_json(&format!("{LLM_URL}/v1/chat/completions"), &body).await?;
    let content = resp.get("choices")?.as_array()?.first()?
        .get("message")?.get("content")?.as_str()?;

    // Parse JSON from response
    let clean = content.trim().trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim();
    let decision: Value = serde_json::from_str(clean).ok()?;

    // Validate action
    let action = decision.get("action")?.as_str()?;
    let valid = ["switch_profile", "no_action", "alert", "restart_service", "run_backfill"];
    if !valid.contains(&action) {
        log("WARN", &format!("RUVLLM: invalid action '{action}'"));
        return None;
    }

    // AIDefence: scan reasoning output
    let reasoning = decision.get("reasoning").and_then(|v| v.as_str()).unwrap_or("");
    if !aidefence::is_safe(reasoning) {
        log("WARN", "AIDEFENCE: LLM reasoning flagged, blocking");
        return None;
    }

    let rag_hits = rag_results.map(|r| r.len()).unwrap_or(0);
    let obs = json!({"gpu_util": gpu.util.to_string(), "gpu_temp": gpu.temp.to_string()});
    write_state("last_reasoning", &json!({
        "at": Local::now().to_rfc3339(),
        "observations": obs,
        "decision": decision,
        "rag_hits": rag_hits,
        "security": "passed",
    }));

    Some(decision)
}

use chrono::Timelike;
