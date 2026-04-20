//! Audio capture, VAD (voice activity detection), and Whisper STT.
//!
//! Uses cpal for mic capture, energy-based VAD to detect speech,
//! records to WAV, then calls whisper CLI for transcription.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

const SAMPLE_RATE: u32 = 16000;
const CHANNELS: u16 = 1;
const ENERGY_THRESHOLD: f32 = 0.15;      // RMS energy to detect speech (Mac mic is sensitive)
const SILENCE_TIMEOUT_MS: u64 = 1500;    // Silence after speech = end of utterance
const MIN_SPEECH_MS: u64 = 500;          // Minimum speech duration
const MAX_SPEECH_MS: u64 = 15000;        // Maximum recording length

/// Check if the mic is muted (PulseAudio/PipeWire) or if the control file says paused.
fn is_mic_muted() -> bool {
    // Check control file first (set by tray icon)
    let control = std::path::Path::new("/tmp/ruos-voice-paused");
    if control.exists() {
        return true;
    }

    // Check PulseAudio/PipeWire mic mute state
    if let Ok(output) = std::process::Command::new("pactl")
        .args(["get-source-mute", "@DEFAULT_SOURCE@"])
        .output()
    {
        let out = String::from_utf8_lossy(&output.stdout);
        if out.contains("yes") {
            return true;
        }
    }

    false
}

pub fn list_devices() -> anyhow::Result<()> {
    let host = cpal::default_host();
    println!("Audio input devices:");
    for (i, dev) in host.input_devices()?.enumerate() {
        let name = dev.name().unwrap_or_else(|_| "unknown".into());
        let cfg = dev.default_input_config().ok();
        if let Some(c) = cfg {
            println!("  [{i}] {name} ({} Hz, {} ch)", c.sample_rate().0, c.channels());
        } else {
            println!("  [{i}] {name} (no config)");
        }
    }
    Ok(())
}

pub fn listen_loop(
    device_idx: Option<usize>,
    wake_word: &str,
    model: &str,
    always_on: bool,
    tx: mpsc::Sender<String>,
) {
    let host = cpal::default_host();
    let device = match device_idx {
        Some(idx) => host.input_devices()
            .expect("no input devices")
            .nth(idx)
            .expect("device index out of range"),
        None => host.default_input_device()
            .expect("no default input device"),
    };

    let dev_name = device.name().unwrap_or_else(|_| "unknown".into());
    eprintln!("  Mic: {dev_name}");

    let config = cpal::StreamConfig {
        channels: CHANNELS,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let buf_clone = buffer.clone();

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = buf_clone.lock().unwrap();
            buf.extend_from_slice(data);
        },
        |err| eprintln!("Audio error: {err}"),
        None,
    ).expect("failed to build input stream");

    stream.play().expect("failed to start audio stream");
    eprintln!("  Audio stream started");

    let mut listening = always_on;
    let wake_lower = wake_word.to_lowercase();
    let whisper_model = model.to_string();

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Check if mic is muted — sleep if so
        if is_mic_muted() {
            // Clear buffer while muted
            buffer.lock().unwrap().clear();
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        let samples = {
            let mut buf = buffer.lock().unwrap();
            let s = buf.clone();
            buf.clear();
            s
        };

        if samples.is_empty() { continue; }

        // Calculate RMS energy
        let energy = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

        if energy < ENERGY_THRESHOLD {
            continue; // Silence — skip
        }

        // Speech detected — record until silence
        eprintln!("  [VAD] Speech detected (energy={energy:.4})");
        let mut recording: Vec<f32> = samples;
        let mut silence_count = 0u64;
        let start = std::time::Instant::now();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let chunk = {
                let mut buf = buffer.lock().unwrap();
                let s = buf.clone();
                buf.clear();
                s
            };

            if chunk.is_empty() {
                silence_count += 100;
            } else {
                let e = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
                if e < ENERGY_THRESHOLD {
                    silence_count += 100;
                } else {
                    silence_count = 0;
                }
                recording.extend_from_slice(&chunk);
            }

            let elapsed = start.elapsed().as_millis() as u64;
            if silence_count > SILENCE_TIMEOUT_MS && elapsed > MIN_SPEECH_MS {
                break; // End of utterance
            }
            if elapsed > MAX_SPEECH_MS {
                break; // Max recording length
            }
        }

        let duration = start.elapsed().as_millis();
        if duration < MIN_SPEECH_MS as u128 { continue; }

        eprintln!("  [VAD] Recorded {duration}ms ({} samples)", recording.len());

        // Write WAV to temp file
        let wav_path = std::env::temp_dir().join("ruos-voice-chunk.wav");
        if write_wav(&wav_path, &recording).is_err() {
            eprintln!("  [WAV] Failed to write");
            continue;
        }

        // Transcribe with Whisper
        eprintln!("  [STT] Transcribing...");
        let text = transcribe(&wav_path, &whisper_model);

        // Cooldown after transcription to save CPU
        std::thread::sleep(std::time::Duration::from_secs(2));
        buffer.lock().unwrap().clear(); // discard audio during cooldown

        if text.is_empty() {
            eprintln!("  [STT] (empty result)");
            continue;
        }

        eprintln!("  [STT] \"{text}\"");

        // Wake word check
        let text_lower = text.to_lowercase();
        if !listening && !always_on {
            if text_lower.contains(&wake_lower) || text_lower.contains("jarvis") {
                listening = true;
                eprintln!("  [WAKE] Activated!");
                let _ = tx.blocking_send("".to_string()); // trigger ready sound
                continue;
            }
            continue; // Not activated yet
        }

        // Send transcription for processing
        if !text_lower.is_empty() {
            let _ = tx.blocking_send(text);
            // Reset to wake-word mode after processing (unless always_on)
            if !always_on {
                listening = false;
            }
        }
    }
}

fn write_wav(path: &std::path::Path, samples: &[f32]) -> anyhow::Result<()> {
    let spec = hound::WavSpec {
        channels: CHANNELS,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in samples {
        writer.write_sample((s * 32767.0) as i16)?;
    }
    writer.finalize()?;
    Ok(())
}

fn transcribe(wav_path: &std::path::Path, model: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/finn".to_string());
    // Use -t 2 to limit CPU (don't max all cores on a laptop)
    let whisper_cmds = [
        format!("whisper-cpp -t 2 -m {home}/.local/share/whisper/ggml-{model}.bin -f {wav} -np -nt",
            wav = wav_path.display()),
        format!("whisper-cpp -t 2 -m /usr/share/whisper-cpp/models/ggml-{model}.bin -f {wav} -np -nt",
            wav = wav_path.display()),
    ];

    for cmd in &whisper_cmds {
        if let Ok(output) = std::process::Command::new("sh")
            .args(["-c", cmd])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|l| !l.trim().is_empty() && !l.starts_with('['))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    return text;
                }
            }
        }
    }

    String::new()
}
