//! ruos-agent — agentic heartbeat daemon for ruOS (pure Rust)
//!
//! Observe-reason-decide-act loop: self-monitor, auto-profile, AIDefence,
//! embed backfill, self-eval, OTA updates, session distillation.

mod aidefence;
mod monitor;
mod profile;
mod backfill;
mod eval;
mod ota;
mod llm;

use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Instant;

const BRAIN_URL: &str = "http://127.0.0.1:9876";
const EMBEDDER_URL: &str = "http://127.0.0.1:9877";
const LLM_URL: &str = "http://127.0.0.1:8080";
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn state_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".local/state/ruos-agent")
}

fn log_file() -> PathBuf {
    state_dir().join("agent.log")
}

pub fn log(level: &str, msg: &str) {
    let ts = Local::now().format("%Y-%m-%dT%H:%M:%S");
    let line = format!("[{ts}] [{level}] {msg}");
    eprintln!("{line}");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true).append(true).open(log_file())
    {
        use std::io::Write;
        let _ = writeln!(f, "{line}");
    }
}

pub fn read_state(key: &str) -> Option<Value> {
    let path = state_dir().join(format!("{key}.json"));
    std::fs::read_to_string(path).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

pub fn write_state(key: &str, data: &Value) {
    let _ = std::fs::create_dir_all(state_dir());
    let path = state_dir().join(format!("{key}.json"));
    let _ = std::fs::write(path, serde_json::to_string_pretty(data).unwrap_or_default());
}

pub async fn http_get_json(url: &str) -> Option<Value> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build().ok()?
        .get(url).send().await.ok()?
        .json().await.ok()
}

pub async fn http_post_json(url: &str, body: &Value) -> Option<Value> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build().ok()?
        .post(url).json(body).send().await.ok()?
        .json().await.ok()
}

// ─── CLI ────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "ruos-agent", version = VERSION, about = "ruOS agentic heartbeat daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Only run health checks + restarts
    #[arg(long)]
    monitor_only: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Show agent state
    Status,
    /// Run self-evaluation
    Eval,
    /// Embed unvectorized memories
    Backfill {
        #[arg(long, default_value = "100")]
        max: usize,
    },
    /// Force DPO training now
    Train,
    /// Check for OTA updates
    Update {
        /// Only check, don't install
        #[arg(long)]
        check: bool,
        /// Force install even if same version
        #[arg(long)]
        force: bool,
    },
    /// AIDefence security
    Security {
        #[command(subcommand)]
        action: SecurityAction,
    },
    /// Store a session insight
    Distill {
        /// The insight text
        text: String,
    },
}

#[derive(Subcommand)]
enum SecurityAction {
    /// Show AIDefence status
    Status,
    /// Run threat detection test suite
    Test,
    /// Scan arbitrary text
    Scan { text: String },
}

// ─── Main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let _ = std::fs::create_dir_all(state_dir());
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Status) => status().await,
        Some(Commands::Eval) => {
            let result = eval::self_eval().await;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Some(Commands::Backfill { max }) => {
            let n = backfill::embed_backfill(max).await;
            println!("Embedded {n} memories");
        }
        Some(Commands::Train) => {
            log("INFO", "Training not yet ported to Rust — use ml-env for now");
        }
        Some(Commands::Update { check, force }) => {
            ota::update(check, force).await;
        }
        Some(Commands::Security { action }) => match action {
            SecurityAction::Status => aidefence::status(),
            SecurityAction::Test => aidefence::test_suite(),
            SecurityAction::Scan { text } => {
                let result = aidefence::scan(&text);
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
        Some(Commands::Distill { text }) => {
            distill(&text).await;
        }
        None => {
            heartbeat(cli.monitor_only).await;
        }
    }

    Ok(())
}

// ─── Heartbeat ──────────────────────────────────────────────────────────────

async fn heartbeat(monitor_only: bool) {
    let start = Instant::now();
    log("INFO", "=== ruos-agent heartbeat ===");

    let mut actions: Vec<String> = Vec::new();

    // 1. Always: self-monitor
    let issues = monitor::self_monitor().await;
    if !issues.is_empty() {
        actions.push(format!("monitor: {}", issues.join(", ")));
    }

    if monitor_only {
        let elapsed = start.elapsed().as_secs_f64();
        log("INFO", &format!("=== heartbeat complete: {} actions in {elapsed:.1}s ===", actions.len()));
        write_state("last_run", &json!({
            "at": Local::now().to_rfc3339(),
            "actions": actions,
            "elapsed_sec": format!("{elapsed:.2}"),
        }));
        return;
    }

    // 2. Auto-profile (with optional LLM reasoning)
    if let Some(result) = profile::auto_profile().await {
        if let Some(rec) = result.get("recommended").and_then(|v| v.as_str()) {
            if !rec.is_empty() {
                actions.push(format!("profile: → {rec}"));
            }
        }
    }

    // 3. Embed backfill (if GPU idle)
    let gpu_util = profile::gpu_util().await.unwrap_or(100);
    if gpu_util < 30 {
        let n = backfill::embed_backfill(20).await;
        if n > 0 {
            actions.push(format!("backfill: embedded {n} memories"));
        }
    }

    // 4. Self-eval (once per hour)
    let run_eval = match read_state("last_eval") {
        Some(v) => {
            let last = v.get("at").and_then(|v| v.as_str()).unwrap_or("");
            chrono::DateTime::parse_from_rfc3339(last)
                .map(|dt| Local::now().signed_duration_since(dt).num_minutes() > 60)
                .unwrap_or(true)
        }
        None => true,
    };
    if run_eval {
        let eval_result = eval::self_eval().await;
        let score = eval_result.get("avg_top_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        actions.push(format!("eval: avg_score={score:.4}"));
    }

    let elapsed = start.elapsed().as_secs_f64();
    log("INFO", &format!("=== heartbeat complete: {} actions in {elapsed:.1}s ===", actions.len()));
    write_state("last_run", &json!({
        "at": Local::now().to_rfc3339(),
        "actions": actions,
        "elapsed_sec": format!("{elapsed:.2}"),
    }));
}

// ─── Status ─────────────────────────────────────────────────────────────────

async fn status() {
    println!("ruos-agent v{VERSION} (Rust)");
    println!("{}", "=".repeat(50));

    if let Some(v) = read_state("last_run") {
        println!("  Last run: {}", v.get("at").and_then(|v| v.as_str()).unwrap_or("?"));
        println!("  Actions:  {:?}", v.get("actions"));
        println!("  Elapsed:  {}s", v.get("elapsed_sec").and_then(|v| v.as_str()).unwrap_or("?"));
    }
    if let Some(v) = read_state("last_eval") {
        println!("  Avg score: {}", v.get("avg_top_score").and_then(|v| v.as_f64()).unwrap_or(0.0));
    }
    if let Some(v) = read_state("last_profile_switch") {
        println!("  Profile: {} → {}",
            v.get("from").and_then(|v| v.as_str()).unwrap_or("?"),
            v.get("to").and_then(|v| v.as_str()).unwrap_or("?"));
    }
    if let Some(v) = read_state("last_reasoning") {
        let empty = json!({});
        let d = v.get("decision").unwrap_or(&empty);
        println!("  LLM: action={} reasoning={}",
            d.get("action").and_then(|v| v.as_str()).unwrap_or("?"),
            &d.get("reasoning").and_then(|v| v.as_str()).unwrap_or("?")[..80.min(
                d.get("reasoning").and_then(|v| v.as_str()).unwrap_or("").len()
            )]);
    }

    println!();
    for (name, url) in [("brain", BRAIN_URL), ("embedder", EMBEDDER_URL), ("ruvllm", LLM_URL)] {
        let ok = http_get_json(&format!("{url}/health")).await.is_some();
        println!("  {name}: {}", if ok { "OK" } else { "DOWN" });
    }

    // Vector coverage
    let db_path = dirs::home_dir().unwrap_or_default().join("brain-data/brain.sqlite");
    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0)).unwrap_or(0);
        let vecs: i64 = conn.query_row("SELECT COUNT(*) FROM memories WHERE LENGTH(embedding) > 0", [], |r| r.get(0)).unwrap_or(0);
        if total > 0 {
            println!("  Vectors: {vecs}/{total} ({}%)", vecs * 100 / total);
        }
    }
}

// ─── Distill ────────────────────────────────────────────────────────────────

async fn distill(text: &str) {
    if text.len() < 10 {
        eprintln!("Text too short");
        return;
    }

    // AIDefence scan before storing
    let scan = aidefence::scan(text);
    if !scan.get("safe").and_then(|v| v.as_bool()).unwrap_or(true) {
        log("WARN", &format!("AIDEFENCE: blocked distill (threat={})",
            scan.get("threat_level").and_then(|v| v.as_str()).unwrap_or("?")));
        return;
    }

    let body = json!({"category": "session-learning", "content": text});
    if let Some(resp) = http_post_json(&format!("{BRAIN_URL}/memories"), &body).await {
        if let Some(id) = resp.get("id").and_then(|v| v.as_str()) {
            log("INFO", &format!("DISTILL: stored → {}", &id[..12.min(id.len())]));
        }
    }
}
