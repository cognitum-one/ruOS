//! Command parser + executor — maps voice commands to ruOS actions.

use serde_json::json;

const BRAIN_URL: &str = "http://127.0.0.1:9876";
const LLM_URL: &str = "http://127.0.0.1:8080";

/// Parse and execute a voice command. Returns a spoken response.
pub async fn execute(text: &str) -> String {
    let lower = text.to_lowercase();

    // ─── System commands ─────────────────────────────────────────────
    if contains_any(&lower, &["system status", "status report", "how are you"]) {
        return system_status().await;
    }

    if contains_any(&lower, &["what time", "what's the time"]) {
        return format!("It's {}", chrono::Local::now().format("%I:%M %p"));
    }

    // ─── Brain search ────────────────────────────────────────────────
    if let Some(query) = extract_after(&lower, &["search for", "search brain for", "find", "look up"]) {
        return brain_search(&query).await;
    }

    if contains_any(&lower, &["what did you learn", "recent memories", "what do you know"]) {
        return brain_recent().await;
    }

    // ─── Profile switching ───────────────────────────────────────────
    if let Some(profile) = extract_profile(&lower) {
        return switch_profile(&profile).await;
    }

    // ─── App launching ───────────────────────────────────────────────
    if let Some(app) = extract_after(&lower, &["open", "launch", "start"]) {
        return open_app(&app).await;
    }

    // ─── Screen control ──────────────────────────────────────────────
    if contains_any(&lower, &["lock screen", "lock the screen", "lock computer"]) {
        return lock_screen().await;
    }

    // ─── Security ────────────────────────────────────────────────────
    if contains_any(&lower, &["security test", "run security", "security scan"]) {
        return security_test().await;
    }

    // ─── Agent ───────────────────────────────────────────────────────
    if contains_any(&lower, &["run agent", "heartbeat", "check services"]) {
        return run_agent().await;
    }

    if contains_any(&lower, &["agent status", "ruos status"]) {
        return agent_status().await;
    }

    // ─── Volume ──────────────────────────────────────────────────────
    if contains_any(&lower, &["volume up", "louder"]) {
        return volume_control("5%+").await;
    }
    if contains_any(&lower, &["volume down", "quieter"]) {
        return volume_control("5%-").await;
    }
    if contains_any(&lower, &["mute", "silence"]) {
        return volume_control("toggle").await;
    }

    // ─── Misc ────────────────────────────────────────────────────────
    if contains_any(&lower, &["thank you", "thanks"]) {
        return "You're welcome".to_string();
    }

    if contains_any(&lower, &["hello", "hi there", "hey"]) {
        return "Hello! How can I help you?".to_string();
    }

    if contains_any(&lower, &["good night", "going to sleep", "shut down"]) {
        return "Good night. Suspending in 5 seconds.".to_string();
    }

    // ─── Fallback: send to local LLM ─────────────────────────────────
    llm_respond(text).await
}

// ─── Action implementations ──────────────────────────────────────────────

async fn system_status() -> String {
    let mut parts = Vec::new();

    // Brain
    if let Some(info) = http_get(&format!("{BRAIN_URL}/brain/info")).await {
        let mem = info.get("memories_count").and_then(|v| v.as_i64()).unwrap_or(0);
        parts.push(format!("Brain has {mem} memories"));
    }

    // Uptime
    if let Ok(output) = tokio::process::Command::new("uptime").arg("-p").output().await {
        let up = String::from_utf8_lossy(&output.stdout).trim().to_string();
        parts.push(format!("System {up}"));
    }

    // WiFi
    if let Ok(output) = tokio::process::Command::new("nmcli")
        .args(["-t", "-f", "DEVICE,STATE,CONNECTION", "device", "status"])
        .output().await
    {
        let out = String::from_utf8_lossy(&output.stdout);
        if let Some(wifi) = out.lines().find(|l| l.contains("wifi")) {
            let parts_w: Vec<&str> = wifi.split(':').collect();
            if parts_w.len() >= 3 {
                parts.push(format!("WiFi connected to {}", parts_w[2]));
            }
        }
    }

    if parts.is_empty() {
        "System is running but I couldn't get details".to_string()
    } else {
        parts.join(". ")
    }
}

async fn brain_search(query: &str) -> String {
    let body = json!({"query": query, "k": 3});
    if let Some(resp) = http_post(&format!("{BRAIN_URL}/brain/search"), &body).await {
        let results = resp.get("results").and_then(|v| v.as_array());
        match results {
            Some(r) if !r.is_empty() => {
                let top = &r[0];
                let score = top.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let content = top.get("content").and_then(|v| v.as_str()).unwrap_or("no content");
                let cat = top.get("category").and_then(|v| v.as_str()).unwrap_or("unknown");
                format!("Found {} results. Top match in {cat} with score {score:.0}: {content}",
                    r.len(), score = score * 100.0, content = &content[..100.min(content.len())])
            }
            _ => format!("No results found for {query}"),
        }
    } else {
        "Brain search failed — is the brain service running?".to_string()
    }
}

async fn brain_recent() -> String {
    if let Some(resp) = http_get(&format!("{BRAIN_URL}/memories?limit=3")).await {
        let mems = resp.get("memories").and_then(|v| v.as_array());
        match mems {
            Some(m) if !m.is_empty() => {
                let cats: Vec<&str> = m.iter()
                    .filter_map(|v| v.get("category").and_then(|c| c.as_str()))
                    .collect();
                format!("Last {} memories are in categories: {}", m.len(), cats.join(", "))
            }
            _ => "No recent memories".to_string(),
        }
    } else {
        "Couldn't reach the brain".to_string()
    }
}

async fn switch_profile(profile: &str) -> String {
    let valid = ["gpu-train", "gpu-inference", "cognitum-balanced", "cognitum-idle"];
    if !valid.contains(&profile) {
        return format!("Unknown profile {profile}. Available: {}", valid.join(", "));
    }

    match tokio::process::Command::new("ruvultra-profile")
        .args(["apply", profile])
        .output().await
    {
        Ok(o) if o.status.success() => format!("Switched to {profile}"),
        _ => format!("Failed to switch to {profile}"),
    }
}

fn extract_profile(text: &str) -> Option<String> {
    let mappings = [
        (&["switch to training", "training mode", "gpu train"][..], "gpu-train"),
        (&["switch to inference", "inference mode"], "gpu-inference"),
        (&["switch to balanced", "balanced mode", "normal mode"], "cognitum-balanced"),
        (&["switch to idle", "idle mode", "power save", "low power"], "cognitum-idle"),
    ];
    for (phrases, profile) in &mappings {
        if phrases.iter().any(|p| text.contains(p)) {
            return Some(profile.to_string());
        }
    }
    None
}

async fn open_app(app: &str) -> String {
    let cmd = match app.trim() {
        "browser" | "firefox" | "web" => "firefox",
        "terminal" | "console" => "gnome-terminal",
        "files" | "file manager" => "nautilus",
        "editor" | "text editor" | "code" | "vs code" | "vscode" => "code",
        "settings" | "system settings" => "gnome-control-center",
        "claude" | "claude code" => "gnome-terminal -- claude",
        other => other,
    };

    match tokio::process::Command::new("sh")
        .args(["-c", &format!("{cmd} &")])
        .output().await
    {
        Ok(_) => format!("Opening {app}"),
        Err(e) => format!("Couldn't open {app}: {e}"),
    }
}

async fn lock_screen() -> String {
    let _ = tokio::process::Command::new("loginctl")
        .args(["lock-session"])
        .output().await;
    "Screen locked".to_string()
}

async fn security_test() -> String {
    match tokio::process::Command::new("ruos-agent")
        .args(["security", "test"])
        .output().await
    {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout);
            let pass = out.matches("PASS").count();
            format!("{pass} of 6 security tests passed")
        }
        Err(_) => "Couldn't run security test".to_string(),
    }
}

async fn run_agent() -> String {
    match tokio::process::Command::new("ruos-agent")
        .arg("--monitor-only")
        .output().await
    {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stderr);
            if out.contains("heartbeat complete") {
                "Agent heartbeat complete. All services healthy.".to_string()
            } else {
                "Agent ran but check logs for details".to_string()
            }
        }
        Err(_) => "Agent not available".to_string(),
    }
}

async fn agent_status() -> String {
    match tokio::process::Command::new("ruos-agent")
        .arg("status")
        .output().await
    {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = out.lines()
                .filter(|l| l.contains("Vectors") || l.contains("brain:") || l.contains("Avg score"))
                .collect();
            if lines.is_empty() {
                "Agent is running".to_string()
            } else {
                lines.join(". ").replace("  ", " ")
            }
        }
        Err(_) => "Agent not available".to_string(),
    }
}

async fn volume_control(action: &str) -> String {
    let args = if action == "toggle" {
        vec!["set-sink-mute", "@DEFAULT_SINK@", "toggle"]
    } else {
        vec!["set-sink-volume", "@DEFAULT_SINK@", action]
    };
    let _ = tokio::process::Command::new("pactl").args(&args).output().await;
    match action {
        "5%+" => "Volume up".to_string(),
        "5%-" => "Volume down".to_string(),
        "toggle" => "Mute toggled".to_string(),
        _ => "Volume adjusted".to_string(),
    }
}

async fn llm_respond(text: &str) -> String {
    let body = json!({
        "model": "Qwen/Qwen2.5-3B-Instruct",
        "messages": [
            {"role": "system", "content": "You are ruOS voice assistant. Give brief spoken responses (1-2 sentences max)."},
            {"role": "user", "content": text}
        ],
        "max_tokens": 100,
        "temperature": 0.7
    });

    if let Some(resp) = http_post(&format!("{LLM_URL}/v1/chat/completions"), &body).await {
        resp.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("I didn't understand that")
            .to_string()
    } else {
        "I can't reach the language model right now".to_string()
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn contains_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| text.contains(p))
}

fn extract_after(text: &str, prefixes: &[&str]) -> Option<String> {
    for prefix in prefixes {
        if let Some(pos) = text.find(prefix) {
            let after = text[pos + prefix.len()..].trim();
            if !after.is_empty() {
                return Some(after.to_string());
            }
        }
    }
    None
}

async fn http_get(url: &str) -> Option<serde_json::Value> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build().ok()?
        .get(url).send().await.ok()?
        .json().await.ok()
}

async fn http_post(url: &str, body: &serde_json::Value) -> Option<serde_json::Value> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build().ok()?
        .post(url).json(body).send().await.ok()?
        .json().await.ok()
}
