//! ruos-voice — Jarvis-style voice control daemon for ruOS
//!
//! Listens for commands via microphone, transcribes with Whisper (via CLI),
//! parses intent, executes actions, responds with TTS.
//!
//! Architecture:
//!   Mic → VAD (energy) → Record chunk → Whisper STT → Parse → Execute → TTS
//!
//! Commands:
//!   "hey ruOS" / "jarvis"           → wake word (activates listening)
//!   "search for [query]"            → brain semantic search
//!   "switch to [profile]"           → change GPU/system profile
//!   "system status"                 → report services, GPU, brain
//!   "what did you learn"            → recent brain memories
//!   "open [app]"                    → launch application
//!   "lock" / "unlock"               → lock/unlock screen
//!   "run security test"             → AIDefence test suite
//!   "[anything else]"               → send to local LLM for response

mod commands;
mod audio;
mod tts;

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::mpsc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "ruos-voice", version = VERSION, about = "Jarvis-style voice control")]
struct Cli {
    /// Microphone device index (default: system default)
    #[arg(long)]
    device: Option<usize>,

    /// Wake word (default: "hey ruos")
    #[arg(long, default_value = "hey ruos")]
    wake_word: String,

    /// Whisper model size (tiny, base, small)
    #[arg(long, default_value = "base")]
    model: String,

    /// Continuous listening (no wake word needed)
    #[arg(long)]
    always_on: bool,

    /// Disable TTS responses
    #[arg(long)]
    silent: bool,

    /// Test mode: process a text command directly
    #[arg(long)]
    test: Option<String>,

    /// List audio devices
    #[arg(long)]
    list_devices: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.list_devices {
        audio::list_devices()?;
        return Ok(());
    }

    // Test mode: skip audio, process a command directly
    if let Some(text) = cli.test {
        println!("Processing: \"{text}\"");
        let response = commands::execute(&text).await;
        println!("Response: {response}");
        if !cli.silent {
            tts::speak(&response);
        }
        return Ok(());
    }

    println!("╔══════════════════════════════════════════════╗");
    println!("║  ruOS Voice Control v{VERSION}                  ║");
    println!("║  Wake word: \"{}\"{}║",
        cli.wake_word,
        " ".repeat(30 - cli.wake_word.len().min(29)));
    println!("║  Model: whisper-{}                          ║", cli.model);
    println!("╚══════════════════════════════════════════════╝");
    println!();

    if !cli.silent {
        tts::speak("ruOS voice control online");
    }

    // Channel for transcribed text
    let (tx, mut rx) = mpsc::channel::<String>(16);

    // Audio capture + VAD + Whisper in background
    let wake_word = Arc::new(cli.wake_word.clone());
    let model = cli.model.clone();
    let always_on = cli.always_on;
    let device = cli.device;

    tokio::task::spawn_blocking(move || {
        audio::listen_loop(device, &wake_word, &model, always_on, tx);
    });

    // Process commands as they arrive
    println!("Listening... (Ctrl+C to stop)");
    while let Some(text) = rx.recv().await {
        let trimmed = text.trim().to_lowercase();
        if trimmed.is_empty() { continue; }

        println!("\n  Heard: \"{trimmed}\"");
        let response = commands::execute(&trimmed).await;
        println!("  → {response}");

        if !cli.silent {
            tts::speak(&response);
        }
    }

    Ok(())
}
