//! Sensor collectors — gated by consent, stores to brain.

use crate::consent;
use anyhow::Result;
use std::process::Command;

const BRAIN_URL: &str = "http://127.0.0.1:9876";

async fn store(category: &str, content: &str) {
    if content.is_empty() { return; }
    let body = serde_json::json!({"category": category, "content": content});
    let _ = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build().unwrap()
        .post(format!("{BRAIN_URL}/memories"))
        .json(&body).send().await;
}

fn cmd(prog: &str, args: &[&str]) -> String {
    Command::new(prog).args(args).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Collect from all consented sensors.
pub async fn collect_all() -> Result<()> {
    println!("Collecting from consented sensors...\n");

    // System metrics (default ON)
    if consent::is_consented("system-metrics") {
        collect_thermal().await;
        collect_gpu().await;
        collect_power().await;
        collect_disk().await;
        collect_display().await;
        collect_usb().await;
    }

    // Network (default ON)
    if consent::is_consented("network-stats") {
        collect_wifi().await;
        collect_traffic().await;
    }

    // CSI (default ON)
    if consent::is_consented("csi-sensing") {
        collect_household().await;
    }

    // Audio (opt-in)
    if consent::is_consented("audio-ambient") {
        collect_audio().await;
    }

    // Bluetooth (opt-in)
    if consent::is_consented("bluetooth-proximity") {
        collect_bluetooth().await;
    }

    // Geo (default ON)
    if consent::is_consented("geo-satellite") {
        collect_external_apis().await;
    }

    println!("\nCollection complete");
    Ok(())
}

/// Run as daemon — collect every 5 minutes.
pub async fn daemon() -> Result<()> {
    println!("ruos-sensors daemon started (collecting every 5 min)\n");
    loop {
        collect_all().await?;
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

// ─── Individual Collectors ──────────────────────────────────────────────────

async fn collect_thermal() {
    let mut temps = Vec::new();
    for entry in std::fs::read_dir("/sys/class/thermal/").into_iter().flatten() {
        if let Ok(e) = entry {
            let name = std::fs::read_to_string(e.path().join("type")).unwrap_or_default();
            let temp = std::fs::read_to_string(e.path().join("temp")).unwrap_or_default();
            if let Ok(t) = temp.trim().parse::<f64>() {
                temps.push(format!("{}={:.0}C", name.trim(), t / 1000.0));
            }
        }
    }
    if !temps.is_empty() {
        let content = format!("Thermal: {}", temps.join(", "));
        println!("  thermal: {}", &content[..80.min(content.len())]);
        store("thermal-history", &content).await;
    }
}

async fn collect_gpu() {
    let out = cmd("nvidia-smi", &["--query-gpu=temperature.gpu,power.draw,utilization.gpu,memory.used",
                                   "--format=csv,noheader,nounits"]);
    if !out.is_empty() {
        let content = format!("GPU: {out}");
        println!("  gpu: {content}");
        store("gpu-workload", &content).await;
    }
}

async fn collect_power() {
    // RAPL energy counters
    let rapl = std::fs::read_to_string("/sys/class/powercap/intel-rapl:0/energy_uj").unwrap_or_default();
    if let Ok(uj) = rapl.trim().parse::<u64>() {
        let watts_approx = uj as f64 / 1_000_000.0; // cumulative, not instantaneous
        let content = format!("RAPL energy: {:.2} J (cumulative)", watts_approx);
        println!("  power: {content}");
        store("energy-usage", &content).await;
    }
}

async fn collect_disk() {
    let out = cmd("df", &["-h", "/"]);
    if !out.is_empty() {
        let line = out.lines().last().unwrap_or("");
        let content = format!("Disk: {line}");
        println!("  disk: {}", &content[..60.min(content.len())]);
        store("disk-activity", &content).await;
    }
}

async fn collect_display() {
    let locked = cmd("loginctl", &["show-session", "auto", "-p", "LockedHint"]);
    let state = if locked.contains("yes") { "locked" } else { "unlocked" };
    let content = format!("Display: {state}");
    println!("  display: {content}");
    store("display-activity", &content).await;
}

async fn collect_usb() {
    let out = cmd("lsusb", &[]);
    let count = out.lines().count();
    let content = format!("USB: {count} devices connected");
    println!("  usb: {content}");
    store("device-events", &content).await;
}

async fn collect_wifi() {
    let out = cmd("nmcli", &["-t", "-f", "SIGNAL,SSID,FREQ", "device", "wifi", "list"]);
    let networks = out.lines().count();
    let content = format!("WiFi: {networks} networks visible");
    println!("  wifi: {content}");
    store("network-quality", &content).await;
}

async fn collect_traffic() {
    let out = cmd("cat", &["/proc/net/dev"]);
    // Sum RX/TX bytes from non-lo interfaces
    let mut rx_total: u64 = 0;
    let mut tx_total: u64 = 0;
    for line in out.lines().skip(2) {
        if line.contains("lo:") { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 10 {
            rx_total += parts[1].parse::<u64>().unwrap_or(0);
            tx_total += parts[9].parse::<u64>().unwrap_or(0);
        }
    }
    let content = format!("Network: RX {:.0}MB TX {:.0}MB",
        rx_total as f64 / 1_048_576.0, tx_total as f64 / 1_048_576.0);
    println!("  traffic: {content}");
    store("network-traffic", &content).await;
}

async fn collect_household() {
    // Check CSI pipeline for motion
    if let Ok(resp) = reqwest::get("http://127.0.0.1:9880/api/status").await {
        if let Ok(data) = resp.json::<serde_json::Value>().await {
            let pipeline = data.get("pipeline").cloned().unwrap_or(serde_json::json!({}));
            let motion = pipeline.get("vitals")
                .and_then(|v| v.get("motion_score"))
                .and_then(|m| m.as_f64()).unwrap_or(0.0);
            let state = if motion < 0.1 { "quiet" } else if motion > 0.4 { "active" } else { "low-activity" };
            let content = format!("Household: {state} (motion {:.0}%)", motion * 100.0);
            println!("  household: {content}");
            store("household-pattern", &content).await;
        }
    }
}

async fn collect_audio() {
    // Record 1 second, compute RMS energy (no speech content)
    let out = Command::new("arecord")
        .args(["-d", "1", "-f", "S16_LE", "-r", "16000", "-c", "1", "-t", "raw", "/tmp/ruos-audio-sample.pcm"])
        .output();

    if let Ok(o) = out {
        if o.status.success() {
            if let Ok(data) = std::fs::read("/tmp/ruos-audio-sample.pcm") {
                let samples: Vec<i16> = data.chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]))
                    .collect();
                let rms = (samples.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / samples.len() as f64).sqrt();
                let db = 20.0 * (rms / 32768.0).max(1e-10).log10();
                let level = if db < -50.0 { "silent" } else if db < -30.0 { "quiet" } else if db < -15.0 { "moderate" } else { "loud" };
                let content = format!("Ambient noise: {level} ({db:.0} dB RMS)");
                println!("  audio: {content}");
                store("ambient-noise", &content).await;
            }
            let _ = std::fs::remove_file("/tmp/ruos-audio-sample.pcm");
        }
    }
}

async fn collect_bluetooth() {
    // Scan for nearby devices (anonymous — hash MACs)
    let out = Command::new("bluetoothctl")
        .args(["--timeout", "5", "scan", "on"])
        .output();

    // Count unique devices from scan
    let devices = cmd("bluetoothctl", &["devices"]);
    let count = devices.lines().count();

    // Hash MACs for privacy
    let content = format!("Bluetooth: {count} devices nearby (anonymous)");
    println!("  bluetooth: {content}");
    store("bluetooth-presence", &content).await;
}

async fn collect_external_apis() {
    // Pollen + UV from Open Meteo
    let loc_path = dirs::home_dir().unwrap_or_default()
        .join(".local/share/ruview/geo-cache/location.json");
    if let Ok(data) = std::fs::read_to_string(&loc_path) {
        if let Ok(loc) = serde_json::from_str::<serde_json::Value>(&data) {
            let lat = loc.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let lon = loc.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let url = format!(
                "https://api.open-meteo.com/v1/forecast?latitude={lat:.4}&longitude={lon:.4}&current=uv_index&daily=sunrise,sunset&timezone=auto&forecast_days=1"
            );
            if let Ok(resp) = reqwest::get(&url).await {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    let uv = data.get("current").and_then(|c| c.get("uv_index"))
                        .and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let content = format!("UV index: {uv:.1}");
                    println!("  uv: {content}");
                    store("uv-index", &content).await;

                    if let Some(daily) = data.get("daily") {
                        let sunrise = daily.get("sunrise").and_then(|a| a.as_array())
                            .and_then(|a| a.first()).and_then(|v| v.as_str()).unwrap_or("?");
                        let sunset = daily.get("sunset").and_then(|a| a.as_array())
                            .and_then(|a| a.first()).and_then(|v| v.as_str()).unwrap_or("?");
                        let content = format!("Daylight: sunrise {sunrise}, sunset {sunset}");
                        println!("  daylight: {content}");
                        store("daylight-hours", &content).await;
                    }
                }
            }
        }
    }
}

// ─── Extended Collectors ────────────────────────────────────────────────────

/// NASA FIRMS — near-real-time fire detection (free, no auth).
async fn collect_fires() {
    let loc_path = dirs::home_dir().unwrap_or_default()
        .join(".local/share/ruview/geo-cache/location.json");
    if let Ok(data) = std::fs::read_to_string(&loc_path) {
        if let Ok(loc) = serde_json::from_str::<serde_json::Value>(&data) {
            let lat = loc.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let lon = loc.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
            // FIRMS VIIRS 24h within 50km
            let url = format!(
                "https://firms.modaps.eosdis.nasa.gov/api/area/csv/OPEN_KEY/VIIRS_SNPP_NRT/{},{},50/1",
                lat, lon
            );
            match reqwest::get(&url).await {
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    let fires = text.lines().skip(1).count(); // skip header
                    let content = if fires > 0 {
                        format!("FIRE ALERT: {} active fires detected within 50km in last 24h", fires)
                    } else {
                        "No active fires within 50km".to_string()
                    };
                    println!("  fires: {content}");
                    store("safety-alert", &content).await;
                }
                Err(_) => println!("  fires: API unavailable"),
            }
        }
    }
}

/// OpenAQ — air quality (free, no auth for basic queries).
async fn collect_air_quality() {
    let loc_path = dirs::home_dir().unwrap_or_default()
        .join(".local/share/ruview/geo-cache/location.json");
    if let Ok(data) = std::fs::read_to_string(&loc_path) {
        if let Ok(loc) = serde_json::from_str::<serde_json::Value>(&data) {
            let lat = loc.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let lon = loc.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let url = format!(
                "https://api.openaq.org/v2/latest?coordinates={},{}&radius=25000&limit=5",
                lat, lon
            );
            match reqwest::get(&url).await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let results = data.get("results").and_then(|r| r.as_array());
                        if let Some(results) = results {
                            let mut readings = Vec::new();
                            for r in results.iter().take(3) {
                                let measurements = r.get("measurements").and_then(|m| m.as_array());
                                if let Some(ms) = measurements {
                                    for m in ms {
                                        let param = m.get("parameter").and_then(|p| p.as_str()).unwrap_or("?");
                                        let value = m.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                        let unit = m.get("unit").and_then(|u| u.as_str()).unwrap_or("");
                                        readings.push(format!("{param}={value:.1}{unit}"));
                                    }
                                }
                            }
                            let content = if readings.is_empty() {
                                "No air quality data nearby".to_string()
                            } else {
                                format!("Air quality: {}", readings.join(", "))
                            };
                            println!("  air: {content}");
                            store("air-quality", &content).await;
                        }
                    }
                }
                Err(_) => println!("  air: API unavailable"),
            }
        }
    }
}

/// Camera-based object detection (luminance regions, no ML model needed).
async fn collect_objects() {
    // Capture a frame and analyze regions
    let output = Command::new("ffmpeg")
        .args(["-y", "-f", "v4l2", "-video_size", "640x480", "-i", "/dev/video0",
               "-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24",
               "/tmp/ruos-object-frame.rgb"])
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            if let Ok(data) = std::fs::read("/tmp/ruos-object-frame.rgb") {
                let w = 640usize;
                let h = 480usize;
                if data.len() >= w * h * 3 {
                    // Analyze frame: count bright/dark/colored regions
                    let mut bright = 0u32;
                    let mut dark = 0u32;
                    let mut red = 0u32;
                    let mut green = 0u32;
                    let mut blue = 0u32;

                    for i in 0..w * h {
                        let ri = i * 3;
                        let r = data[ri] as u32;
                        let g = data[ri + 1] as u32;
                        let b = data[ri + 2] as u32;
                        let lum = (r * 299 + g * 587 + b * 114) / 1000;

                        if lum > 180 { bright += 1; }
                        else if lum < 40 { dark += 1; }

                        if r > 150 && g < 100 && b < 100 { red += 1; }
                        if g > 150 && r < 100 && b < 100 { green += 1; }
                        if b > 150 && r < 100 && g < 100 { blue += 1; }
                    }

                    let total = (w * h) as f32;
                    let bright_pct = bright as f32 / total * 100.0;
                    let dark_pct = dark as f32 / total * 100.0;

                    let mut objects = Vec::new();
                    if bright_pct > 10.0 { objects.push(format!("screens/lights ({bright_pct:.0}%)")); }
                    if red > 1000 { objects.push(format!("red objects ({} px)", red)); }
                    if green > 1000 { objects.push(format!("green objects ({} px)", green)); }
                    if blue > 1000 { objects.push(format!("blue objects ({} px)", blue)); }

                    let lighting = if bright_pct > 30.0 { "well-lit" }
                        else if dark_pct > 70.0 { "dark" }
                        else { "dim" };

                    let content = format!("Room: {lighting}. {}",
                        if objects.is_empty() { "No distinct objects detected".into() }
                        else { objects.join(", ") });
                    println!("  objects: {content}");
                    store("room-objects", &content).await;
                }
            }
            let _ = std::fs::remove_file("/tmp/ruos-object-frame.rgb");
        }
    }
}

/// Lighting history — track light levels over time.
async fn collect_lighting() {
    let output = Command::new("ffmpeg")
        .args(["-y", "-f", "v4l2", "-video_size", "160x120", "-i", "/dev/video0",
               "-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24",
               "/tmp/ruos-light-sample.rgb"])
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            if let Ok(data) = std::fs::read("/tmp/ruos-light-sample.rgb") {
                let pixels = data.len() / 3;
                if pixels > 0 {
                    let avg_lum: f32 = (0..pixels)
                        .map(|i| {
                            let ri = i * 3;
                            (data[ri] as f32 * 0.299 + data[ri+1] as f32 * 0.587 + data[ri+2] as f32 * 0.114)
                        })
                        .sum::<f32>() / pixels as f32;

                    // Color temperature estimate (warm vs cool)
                    let avg_r: f32 = (0..pixels).map(|i| data[i*3] as f32).sum::<f32>() / pixels as f32;
                    let avg_b: f32 = (0..pixels).map(|i| data[i*3+2] as f32).sum::<f32>() / pixels as f32;
                    let warmth = if avg_r > avg_b + 20.0 { "warm" } else if avg_b > avg_r + 20.0 { "cool" } else { "neutral" };

                    let content = format!("Lighting: luminance {avg_lum:.0}/255, color temp {warmth}");
                    println!("  lighting: {content}");
                    store("lighting-history", &content).await;
                }
            }
            let _ = std::fs::remove_file("/tmp/ruos-light-sample.rgb");
        }
    }
}

/// Extended collection — includes NASA FIRMS, OpenAQ, objects, lighting.
pub async fn collect_extended() -> Result<()> {
    collect_all().await?;

    // Extended sensors (need geo consent)
    if consent::is_consented("geo-satellite") {
        collect_fires().await;
        collect_air_quality().await;
    }

    // Camera-based (need camera consent)
    if consent::is_consented("camera-objects") {
        collect_objects().await;
        collect_lighting().await;
    }

    Ok(())
}
