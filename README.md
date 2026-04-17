# ruOS

The first agentic operating system for AI workstations.

ruOS doesn't just respond to commands — it **observes, reasons, and acts** on its own behalf. A local Qwen2.5-3B model reasons about system state every 15 minutes, using RAG from a brain with 600+ memories. It monitors its own health, switches GPU profiles, trains itself overnight, embeds new knowledge during idle time, scans for prompt injection, senses physical presence via WiFi, and updates itself from GitHub releases. All local. No cloud. No external API calls.

## What Makes It Agentic

Traditional operating systems wait. ruOS decides — with LLM reasoning.

| Capability | What it does | How often |
|-----------|-------------|-----------|
| **LLM reasoning** | Qwen2.5-3B on CUDA reasons about system state via RAG | Every 15 min (or on state change) |
| **Self-monitor** | Detects crashed services, restarts them, warns on GPU thermal | Every 5 min |
| **Auto-profile** | LLM + rules evaluate GPU utilization + time + presence → profile switch | Every 5 min |
| **Embed backfill** | Finds unvectorized memories, embeds them during GPU idle | Every 5 min (when idle) |
| **Nightly training** | Exports preference pairs → DPO fine-tunes a LoRA adapter | Daily 3 AM |
| **Self-evaluation** | Tests search quality on reference queries, detects drift | Hourly |
| **AIDefence security** | Scans brain content + LLM output for injection, PII, jailbreak | Every agent cycle |
| **Sensor context** | Maps ESP32 WiFi presence data to profile decisions | Every 5 min |
| **Session distillation** | Saves Claude Code session insights to brain memory | Post-session |
| **OTA updates** | Checks GitHub releases, downloads + installs new packages | Weekly |

All coordinated by a single daemon: `ruos-agent`.

## Comparison

| Feature | Traditional Linux | ChromeOS | macOS | **ruOS** |
|---------|------------------|----------|-------|----------|
| Self-healing services | systemd restart-on-failure | yes | launchd | **yes + health probes + thermal protection** |
| AI tools built-in | no | Gemini (cloud) | Apple Intelligence (cloud) | **124 MCP tools, all local** |
| Local brain/memory | no | no | no | **600+ memories, DiskANN search, 23ms** |
| Local LLM reasoning | no | no | no | **Qwen2.5-3B on CUDA, 1.2s decisions** |
| Self-training | no | no | no | **DPO overnight, LoRA adapters** |
| AI security (prompt injection) | no | no | no | **AIDefence: 32 patterns, <0.1ms** |
| Auto GPU management | no | N/A | N/A | **6 profiles, LLM-driven auto-switch** |
| Physical sensing | no | no | no | **WiFi CSI via $9 ESP32 nodes** |
| OTA updates | apt/dnf (manual) | yes (auto) | yes (auto) | **yes (auto from GitHub releases)** |
| Agentic identity | no | Google account | Apple ID | **ed25519 node keys** |
| Pre-installed AI IDE | no | no | no | **Claude Code + CLAUDE.md + .mcp.json** |
| Runs without internet | partially | no | partially | **fully (brain, embedder, LLM, training)** |

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│  Claude Code + CLAUDE.md + .mcp.json       (agentic IDE)      │
├────────────────────────────────────────────────────────────────┤
│  ruos-agent        ruos-update        AIDefence               │
│  (heartbeat)       (OTA)              (security guard)        │
├────────────────────────────────────────────────────────────────┤
│  ruos-llm-serve (Qwen2.5-3B, CUDA)   (local LLM reasoning)  │
├────────────────────────────────────────────────────────────────┤
│  102 MCP tools (stdio)  │  22 brain tools (HTTP loopback)     │
├─────────────────────────┴──────────────────────────────────────┤
│  ruvultra-mcp       ruvultra-profile      ruvultra-init       │
│  (Rust, stdio)      (Rust, sudo)          (Rust, CLI)         │
├────────────────────────────────────────────────────────────────┤
│  mcp-brain-server   ruvultra-embedder     train_dpo.py        │
│  (DiskANN + SQLite) (candle-cuda, NVML)   (trl + peft)        │
├────────────────────────────────────────────────────────────────┤
│  brain.rvf          adapters/             profiles/           │
│  (cognitive store)  (LoRA weights)        (system TOML)       │
├────────────────────────────────────────────────────────────────┤
│  ESP32 CSI bridge   ed25519 identity      GPU (CUDA)          │
│  (WiFi sensing)     (node keys)           (RTX/Jetson)        │
├────────────────────────────────────────────────────────────────┤
│  Linux kernel + NVIDIA driver + systemd                       │
└────────────────────────────────────────────────────────────────┘
```

## The Agentic Loop

```
         ┌──────────┐
    ┌───►│ OBSERVE  │ GPU util, temp, processes, presence, time
    │    └────┬─────┘
    │         ▼
    │    ┌──────────┐
    │    │ REASON   │ Qwen2.5-3B + RAG from brain + AIDefence scan
    │    └────┬─────┘
    │         ▼
    │    ┌──────────┐
    │    │  DECIDE  │ Which profile? Train now? Backfill? Alert?
    │    └────┬─────┘
    │         ▼
    │    ┌──────────┐
    └────┤   ACT    │ Switch profile, restart service, embed, train
         └──────────┘
              │
         every 5 min (LLM every 15 min)
```

## 10-Level Stack

| Level | Component | Status | Details |
|-------|-----------|--------|---------|
| 1 | Identity | Done | Ed25519 keys via `ruvultra-init identity` |
| 2 | Brain | Done | RVF + SQLite, 600+ memories, 100% vectorized |
| 3 | Embedder | Done | CUDA bge-small-en-v1.5 (384-d, 2ms) or Intel OpenVINO |
| 4 | Semantic search | Done | DiskANN Vamana graph, avg score 0.77 |
| 5 | MCP tools | Done | 102 stdio + 22 brain = 124 total |
| 6 | System profiles | Done | 6 profiles, atomic apply/rollback, LLM-driven auto-switch |
| 7 | Desktop app | Done | Tauri v2 + Svelte 5, gold neural theme |
| 8 | Contrastive data | Done | 3,265 preference pairs, nightly export |
| 9 | DPO training | Done | LoRA adapter, loss 0.081, 100% eval accuracy |
| 10 | Agent + LLM + Security | Done | Qwen2.5-3B reasoning, AIDefence, OTA updates |

## Packages

| Package | Arch | Size | Description |
|---------|------|------|-------------|
| `ruos-core` | amd64 | 4.0 MB | MCP server (102 tools), profile helper, brain backend (RVF), init tool |
| `ruos-core` | arm64 | 829 KB | Same (no GPU deps) for Pi 5, Jetson, Apple Silicon |
| `ruos-brain-base` | all | 359 KB | Pre-trained brain.rvf — 50 curated memories in RVF format |
| `ruos-embedder` | amd64 | 68 MB | CUDA embedding service (candle + bge-small-en-v1.5, 384-d vectors) |
| `ruos-embedder-intel` | all | 3.6 KB | Intel/OpenVINO embedder — drop-in replacement for CUDA variant |
| `ruos-desktop` | amd64 | 4.8 MB | Tauri desktop dashboard (gold neural theme) |

## Autonomous Services

| Service | Type | Schedule | Purpose |
|---------|------|----------|---------|
| `ruvultra-brain` | Long-running | Always | DiskANN brain backend, semantic search |
| `ruvultra-embedder` | Long-running | Always | CUDA/OpenVINO vector encoder |
| `ruos-llm` | Long-running | Always | Qwen2.5-3B local LLM for agent reasoning |
| `ruvultra-csi-bridge` | Long-running | Always | ESP32 WiFi sensor data → brain |
| `ruos-agent` | Timer | Every 5 min | Agentic heartbeat (observe-reason-decide-act) |
| `ruos-agent-nightly` | Timer | Daily 3 AM | Export + DPO train + self-eval |
| `ruos-update` | Timer | Weekly Sun 4 AM | OTA update check |

## Security (AIDefence)

Built-in AI security layer protects the brain and agent:

- **32 injection patterns**: instruction override, jailbreak, DAN mode, system prompt extraction
- **6 PII detectors**: email, phone, SSN, credit card, IP address, API keys
- **Unicode homoglyph normalization**: Cyrillic spoofing protection
- **RAG scanning**: brain search results scanned before entering LLM context
- **LLM output validation**: agent decisions validated against action allowlist
- **<0.1ms latency**: pure Python regex, no external service

```bash
ruos-agent --security test     # run 6-category threat detection test
ruos-agent --security status   # show guard state + audit stats
ruos-agent --security "text"   # scan arbitrary text
```

## Self-Improvement Loop

The machine learns from its own corrections — fully autonomous:

```
Brain memories ──→ Nightly export ──→ Preference pairs (JSONL)
      ▲                                       │
      │                                       ▼
  ruos-agent                           DPO training (trl + peft)
  backfills vectors                           │
  monitors health                             ▼
  LLM reasons                         LoRA adapter (18 MB)
  AIDefence scans                             │
                                              ▼
                                       Improved inference
```

- **Export**: nightly cron extracts chosen/rejected pairs from brain corrections
- **Train**: `train_dpo.py` fine-tunes TinyLlama-1.1B with LoRA (r=16, beta=0.1)
- **Result**: 100% eval accuracy, reward margin 13.96, loss 0.081 on 205 pairs
- **Schedule**: fully autonomous — export at 3 AM, train, eval, no human needed

## OTA Updates

ruOS updates itself from GitHub releases:

```bash
ruos-update --check     # see if update is available
ruos-update             # download + install latest
ruos-update --force     # reinstall current version
```

- Checks `cognitum-one/ruOS` releases weekly
- Downloads arch-specific `.deb` packages via `gh` CLI
- Stops services → installs → restarts (zero-downtime for brain data)
- Records update event in brain memory for audit trail

## Storage Format

ruOS uses **RVF (RuVector Format)** as the native brain storage:
- Append-only cognitive containers with per-segment XXH3-128 hash chains
- 8 segment types: Memory, Vector, Manifest, Metadata, Delta, Snapshot, Tombstone, Extension
- DiskANN Vamana graph index (brute force <2K vectors, graph search above)
- Ed25519 signing for provenance
- Portable: `cp brain.rvf /media/usb/` is a complete brain backup

## Bootstrap (recommended)

The interactive bootstrapper auto-detects hardware and deploys the right configuration:

```bash
bash scripts/ruos-bootstrap              # interactive wizard
bash scripts/ruos-bootstrap --role workstation   # direct deploy
bash scripts/ruos-bootstrap --status             # show deployment state
```

### Deployment Roles

| Role | Components | Use Case |
|------|-----------|----------|
| `workstation` | Brain + CUDA embedder + Qwen2.5-3B LLM + agent + desktop + Claude Code | GPU workstation (RTX/Quadro) |
| `edge` | Brain + embedder (CPU/Intel) + agent | Pi 5, Jetson, Intel NUC |
| `cluster-primary` | Full stack + QUIC federation server + mDNS discovery | Multi-node primary |
| `cluster-secondary` | Brain + agent, syncs from primary | Multi-node replica |
| `agent-only` | Agent + MCP tools, remote brain | Lightweight headless node |
| `docker` | Generate docker-compose.yml (CPU + GPU variants) | Container deployment |
| `minimal` | Brain + MCP tools only | Smallest footprint |

### Federation / Clustering

```bash
# Node A: primary (serves brain data)
ruos-bootstrap --role cluster-primary

# Node B: secondary (auto-discovers primary via mDNS)
ruos-bootstrap --role cluster-secondary

# Node C: secondary (explicit primary address)
ruos-bootstrap --role cluster-secondary --primary 192.168.1.100:9878
```

Federation uses QUIC for brain sync, ed25519 for node authentication, and
last-writer-wins conflict resolution. See ADR-SYS-0011.

## Install (manual)

### Workstation (GPU)
```bash
sudo dpkg -i ruos-core_*.deb ruos-brain-base_*.deb ruos-embedder_*.deb ruos-desktop_*.deb
ruvultra-init setup
```

### Edge device (Pi 5, Jetson — arm64)
```bash
sudo dpkg -i ruos-core_*_arm64.deb ruos-brain-base_*.deb
ruvultra-init setup --role cognitum-secondary
```

### Docker
```bash
docker compose up -d                                # CPU-only
docker compose -f docker-compose.gpu.yml up -d      # with NVIDIA GPU
```

### USB (offline)
```bash
bash scripts/create-bootable-usb.sh /mnt/usb
# Then on target machine:
bash /media/usb/ruos-installer/install.sh
```

## Build from Source

```bash
make all          # build amd64 + arm64 binaries
make deb          # package .debs
make deb-brain    # package pre-trained brain (RVF)
make deb-embedder # package CUDA embedder + model
make deb-desktop  # package Tauri desktop app
make test-docker  # test amd64 install in Docker
make test-arm64   # test arm64 via QEMU emulation
make release      # build + package + test everything
```

## MCP Tools (124)

The MCP server exposes 102 tools over stdio JSON-RPC 2.0 + 22 brain HTTP tools:

| Category | Count | Examples |
|----------|-------|---------|
| GPU management | 12 | `ruv_gpu_status`, `ruv_gpu_clocks`, `ruv_gpu_mps_*` |
| System profiles | 8 | `ruv_profile_current`, `ruv_profile_apply`, `ruv_profile_rollback` |
| Brain / memory | 22 | `ruv_brain_search`, `ruv_brain_store`, `ruv_brain_export` |
| System info | 15 | `ruv_system_info`, `ruv_services_status`, `ruv_disk_usage` |
| Training | 6 | `ruv_training_stats`, `ruv_training_export_pairs` |
| Auto-profile | 4 | `ruv_auto_profile_workload`, `ruv_auto_profile_apply` |
| Networking | 8 | `ruv_net_interfaces`, `ruv_net_ports` |
| Process management | 6 | `ruv_process_list`, `ruv_process_top` |
| Diagnostics | 11 | `ruv_diag_health`, `ruv_diag_benchmarks` |
| Identity | 4 | `ruv_identity_status`, `ruv_identity_verify` |
| Sensor / ESP32 | 6 | `ruv_sensor_list`, `ruv_sensor_presence`, `ruv_sensor_vitals` |

## CLI Tools

```bash
ruos-bootstrap              # deploy wizard (7 roles)
ruos-bootstrap --status     # show deployment state
ruos-agent                  # run agentic heartbeat now
ruos-agent --status         # show agent state + LLM reasoning
ruos-agent --eval           # run self-evaluation
ruos-agent --backfill       # embed unvectorized memories
ruos-agent --train          # force DPO training now
ruos-agent --security test  # run AIDefence test suite
ruos-update --check         # check for OTA updates
ruvultra-init status        # system overview
ruvultra-profile apply gpu-train  # manual profile switch
```

## Tested Architectures

| Arch | Tier | Performance | Devices |
|------|------|-------------|---------|
| amd64 | 1 (CI) | 1,333 MCP calls/sec, <1ms cold start | Workstations, servers |
| arm64 | 1 (CI) | 27 calls/sec (QEMU emulated) | Pi 5, Jetson, Apple Silicon |
| armv7 | 2 (community) | Base image boots, brain.rvf portable | Pi 3/4 (32-bit) |
| riscv64 | 2 (community) | Base image boots | StarFive, Sipeed |

## WiFi Sensing (RuView)

Optional perception layer using ESP32-S3 nodes ($9 each):
- Presence detection, vital signs (breathing/heart rate), activity recognition
- WiFi CSI (Channel State Information) — no cameras, through walls
- Data flows as RVF segments into the brain for searchable sensor history
- Agent daemon uses presence data for automatic profile switching

## Architecture Decision Records

19 ADRs document every design choice:

| ADR | Title |
|-----|-------|
| 0001 | MCP stdio server architecture |
| 0002 | Local private brain backend |
| 0003 | GPU management via NVML |
| 0004 | Contrastive learning (DPO/IPO preference pairs) |
| 0005 | System profile management with rollback |
| 0006 | Local vector store and embeddings |
| 0007 | Profile-based GPU optimization |
| 0008 | RVF format for brain storage |
| 0009 | Ed25519 identity and signing |
| 0010 | Cross-compilation for arm64 |
| 0011 | QUIC federation protocol |
| 0012 | Desktop app (Tauri + Svelte 5) |
| 0013 | Multi-architecture testing (QEMU) |
| 0014 | RuView WiFi sensing integration |
| 0015 | ruos-agent agentic heartbeat daemon |
| 0016 | DiskANN vector index + Intel OpenVINO embedder |
| 0017 | Local LLM reasoning via Qwen2.5-3B |
| 0018 | AIDefence security layer |

## Links

- Platform: [cognitum-one/ruVultra](https://github.com/cognitum-one/ruVultra) (MCP server, desktop app, ADRs)
- RuView: [ruvnet/RuView](https://github.com/ruvnet/RuView) (WiFi sensing firmware)
- Cognitum: [cognitum.one](https://cognitum.one)

## License

MIT
