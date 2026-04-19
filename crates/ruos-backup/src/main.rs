//! ruos-backup — incremental system backup with rollback
//!
//! Uses rsync hard-link snapshots for space-efficient incremental backups.
//! Each snapshot only stores changed files; unchanged files are hard-linked
//! to the previous snapshot (zero extra disk space).
//!
//! Usage:
//!   ruos-backup snapshot              # create a snapshot now
//!   ruos-backup list                  # list all snapshots
//!   ruos-backup rollback <ID>         # restore a snapshot
//!   ruos-backup rollback --latest     # restore the most recent snapshot
//!   ruos-backup prune --keep 5        # keep last N snapshots, delete rest
//!   ruos-backup status                # show backup state + disk usage
//!   ruos-backup schedule              # install daily timer

use anyhow::{bail, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn backup_dir() -> PathBuf {
    let dir = std::env::var("RUOS_BACKUP_DIR")
        .unwrap_or_else(|_| format!("{}/.local/share/ruos-backup", std::env::var("HOME").unwrap_or("/root".into())));
    PathBuf::from(dir)
}

fn snapshots_dir() -> PathBuf { backup_dir().join("snapshots") }
fn manifest_path() -> PathBuf { backup_dir().join("manifest.json") }

#[derive(Serialize, Deserialize, Clone)]
struct Snapshot {
    id: String,
    created_at: String,
    size_bytes: u64,
    files_changed: u64,
    source: String,
    description: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Manifest {
    snapshots: Vec<Snapshot>,
}

impl Manifest {
    fn load() -> Self {
        std::fs::read_to_string(manifest_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn save(&self) {
        let _ = std::fs::create_dir_all(backup_dir());
        let _ = std::fs::write(manifest_path(), serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

// What to back up
const BACKUP_SOURCES: &[&str] = &[
    "/etc",
    "/home",
    "/usr/local/bin",
    "/usr/local/lib/ruos",
];

const EXCLUDE: &[&str] = &[
    ".cache",
    ".local/share/Trash",
    "node_modules",
    ".npm",
    "__pycache__",
    "*.pyc",
    ".cargo/registry",
    "target/debug",
    "target/release",
    "snap",
];

#[derive(Parser)]
#[command(name = "ruos-backup", version = VERSION, about = "Incremental backup with rollback")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a snapshot now
    Snapshot {
        /// Description for this snapshot
        #[arg(short, long, default_value = "manual snapshot")]
        desc: String,
    },
    /// List all snapshots
    List,
    /// Rollback to a snapshot
    Rollback {
        /// Snapshot ID (or "latest")
        id: String,
        /// Dry run — show what would change
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete old snapshots
    Prune {
        /// Keep the last N snapshots
        #[arg(long, default_value = "5")]
        keep: usize,
    },
    /// Show backup status
    Status,
    /// Install daily backup timer
    Schedule,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Snapshot { desc } => snapshot(&desc)?,
        Commands::List => list()?,
        Commands::Rollback { id, dry_run } => rollback(&id, dry_run)?,
        Commands::Prune { keep } => prune(keep)?,
        Commands::Status => status()?,
        Commands::Schedule => schedule()?,
    }
    Ok(())
}

fn snapshot(desc: &str) -> Result<()> {
    let now = Local::now();
    let id = now.format("%Y%m%d-%H%M%S").to_string();
    let snap_dir = snapshots_dir().join(&id);
    std::fs::create_dir_all(&snap_dir)?;

    println!("Creating snapshot: {id}");
    println!("  Description: {desc}");

    let mut manifest = Manifest::load();
    let latest = manifest.snapshots.last().map(|s| snapshots_dir().join(&s.id));

    let mut total_changed = 0u64;
    for source in BACKUP_SOURCES {
        if !Path::new(source).exists() { continue; }

        let dest = snap_dir.join(source.trim_start_matches('/'));
        std::fs::create_dir_all(&dest)?;

        let mut cmd = Command::new("rsync");
        cmd.args(["-a", "--delete", "--stats"]);

        // Hard-link to previous snapshot for dedup
        if let Some(ref prev) = latest {
            let link_dest = prev.join(source.trim_start_matches('/'));
            if link_dest.exists() {
                cmd.arg(format!("--link-dest={}", link_dest.display()));
            }
        }

        // Excludes
        for exc in EXCLUDE {
            cmd.arg(format!("--exclude={exc}"));
        }

        cmd.arg(format!("{}/", source));
        cmd.arg(format!("{}/", dest.display()));

        let output = cmd.output()?;
        let stats = String::from_utf8_lossy(&output.stdout);

        // Parse changed files count
        if let Some(line) = stats.lines().find(|l| l.contains("files transferred")) {
            if let Some(n) = line.split(':').nth(1).and_then(|s| s.trim().replace(',', "").parse::<u64>().ok()) {
                total_changed += n;
            }
        }

        println!("  {source}: synced");
    }

    // Calculate snapshot size
    let size = dir_size(&snap_dir);

    let snap = Snapshot {
        id: id.clone(),
        created_at: now.to_rfc3339(),
        size_bytes: size,
        files_changed: total_changed,
        source: BACKUP_SOURCES.join(", "),
        description: desc.to_string(),
    };
    manifest.snapshots.push(snap);
    manifest.save();

    println!("\nSnapshot {id} complete");
    println!("  Changed: {total_changed} files");
    println!("  Size: {}", human_size(size));
    Ok(())
}

fn list() -> Result<()> {
    let manifest = Manifest::load();
    if manifest.snapshots.is_empty() {
        println!("No snapshots. Create one: ruos-backup snapshot");
        return Ok(());
    }
    println!("{:<20} {:<12} {:<10} {}", "ID", "Changed", "Size", "Description");
    println!("{}", "-".repeat(65));
    for s in &manifest.snapshots {
        println!("{:<20} {:<12} {:<10} {}",
            s.id, s.files_changed, human_size(s.size_bytes), s.description);
    }
    println!("\nTotal: {} snapshots", manifest.snapshots.len());
    Ok(())
}

fn rollback(id: &str, dry_run: bool) -> Result<()> {
    let manifest = Manifest::load();

    let snap = if id == "latest" || id == "last" {
        manifest.snapshots.last()
    } else {
        manifest.snapshots.iter().find(|s| s.id == id)
    };

    let snap = match snap {
        Some(s) => s.clone(),
        None => bail!("Snapshot '{id}' not found. Run: ruos-backup list"),
    };

    let snap_dir = snapshots_dir().join(&snap.id);
    if !snap_dir.exists() {
        bail!("Snapshot directory missing: {}", snap_dir.display());
    }

    println!("Rolling back to: {}", snap.id);
    println!("  Created: {}", snap.created_at);
    println!("  Description: {}", snap.description);

    if dry_run {
        println!("\n  [DRY RUN] Would restore:");
        for source in BACKUP_SOURCES {
            let src = snap_dir.join(source.trim_start_matches('/'));
            if src.exists() {
                println!("    {} → {}", src.display(), source);
            }
        }
        return Ok(());
    }

    println!("\n  WARNING: This will overwrite current system files!");
    println!("  Press Ctrl+C to cancel, or wait 5 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Create a pre-rollback snapshot first
    println!("\n  Creating pre-rollback snapshot...");
    snapshot("auto: pre-rollback")?;

    // Restore each source
    for source in BACKUP_SOURCES {
        let src = snap_dir.join(source.trim_start_matches('/'));
        if !src.exists() { continue; }

        println!("  Restoring {source}...");
        let status = Command::new("sudo")
            .args(["rsync", "-a", "--delete",
                   &format!("{}/", src.display()),
                   &format!("{}/", source)])
            .status()?;

        if !status.success() {
            eprintln!("    WARNING: rsync failed for {source}");
        }
    }

    println!("\nRollback to {} complete!", snap.id);
    println!("A pre-rollback snapshot was saved.");
    println!("Reboot recommended: sudo reboot");
    Ok(())
}

fn prune(keep: usize) -> Result<()> {
    let mut manifest = Manifest::load();
    if manifest.snapshots.len() <= keep {
        println!("Only {} snapshots, keeping all (keep={})", manifest.snapshots.len(), keep);
        return Ok(());
    }

    let to_remove = manifest.snapshots.len() - keep;
    let removed: Vec<Snapshot> = manifest.snapshots.drain(..to_remove).collect();

    for snap in &removed {
        let dir = snapshots_dir().join(&snap.id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        println!("  Pruned: {} ({})", snap.id, snap.description);
    }

    manifest.save();
    println!("\nRemoved {}, keeping {}", removed.len(), manifest.snapshots.len());
    Ok(())
}

fn status() -> Result<()> {
    let manifest = Manifest::load();
    let total_size = dir_size(&backup_dir());

    println!("ruos-backup v{VERSION}");
    println!("  Backup dir: {}", backup_dir().display());
    println!("  Snapshots: {}", manifest.snapshots.len());
    println!("  Total size: {}", human_size(total_size));
    if let Some(latest) = manifest.snapshots.last() {
        println!("  Latest: {} ({})", latest.id, latest.description);
        println!("  Created: {}", latest.created_at);
    }

    // Check timer
    let timer = Command::new("systemctl")
        .args(["--user", "is-active", "ruos-backup.timer"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());
    println!("  Auto-backup: {}", timer);

    Ok(())
}

fn schedule() -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/finn".into());
    let systemd_dir = format!("{home}/.config/systemd/user");
    std::fs::create_dir_all(&systemd_dir)?;

    // Service
    std::fs::write(format!("{systemd_dir}/ruos-backup.service"), format!(
"[Unit]
Description=ruOS daily backup snapshot

[Service]
Type=oneshot
ExecStart={home}/.local/bin/ruos-backup snapshot -d \"daily auto-backup\"
"))?;

    // Timer — daily at 2 AM
    std::fs::write(format!("{systemd_dir}/ruos-backup.timer"),
"[Unit]
Description=ruOS daily backup at 2 AM

[Timer]
OnCalendar=*-*-* 02:00:00
Persistent=true
RandomizedDelaySec=600

[Install]
WantedBy=timers.target
")?;

    // Prune service — weekly, keep 7
    std::fs::write(format!("{systemd_dir}/ruos-backup-prune.service"), format!(
"[Unit]
Description=ruOS backup prune — keep last 7 snapshots

[Service]
Type=oneshot
ExecStart={home}/.local/bin/ruos-backup prune --keep 7
"))?;

    std::fs::write(format!("{systemd_dir}/ruos-backup-prune.timer"),
"[Unit]
Description=ruOS weekly backup prune

[Timer]
OnCalendar=Sun *-*-* 03:00:00
Persistent=true

[Install]
WantedBy=timers.target
")?;

    Command::new("systemctl").args(["--user", "daemon-reload"]).output()?;
    Command::new("systemctl").args(["--user", "enable", "--now", "ruos-backup.timer"]).output()?;
    Command::new("systemctl").args(["--user", "enable", "--now", "ruos-backup-prune.timer"]).output()?;

    println!("Backup scheduled:");
    println!("  Daily at 2 AM: ruos-backup snapshot");
    println!("  Weekly Sunday 3 AM: prune to keep 7");
    Ok(())
}

fn dir_size(path: &Path) -> u64 {
    Command::new("du").args(["-sb", &path.display().to_string()])
        .output().ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).split_whitespace().next()
            .and_then(|s| s.parse().ok()))
        .unwrap_or(0)
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 { return format!("{bytes}B"); }
    if bytes < 1048576 { return format!("{:.0}K", bytes as f64 / 1024.0); }
    if bytes < 1073741824 { return format!("{:.1}M", bytes as f64 / 1048576.0); }
    format!("{:.1}G", bytes as f64 / 1073741824.0)
}
