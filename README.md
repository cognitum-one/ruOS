# ruOS

The first agentic operating system for AI workstations. Local-first, contrastive-learning-native, self-improving — built for the Claude Code era.

ruOS turns a Linux workstation into a cognitive machine: 124 MCP tools, a local brain with semantic search, real-time GPU management, DPO self-training, and optional WiFi sensing — all running locally with no cloud dependency.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Claude Code + CLAUDE.md + .mcp.json (agentic layer)     │
├──────────────────────────────────────────────────────────┤
│  102 MCP tools (stdio)  │  22 brain tools (HTTP)         │
├─────────────────────────┴────────────────────────────────┤
│  ruvultra-mcp        ruvultra-profile     ruvultra-init  │
│  (Rust, stdio)       (Rust, sudo)         (Rust, CLI)    │
├──────────────────────────────────────────────────────────┤
│  mcp-brain-server    ruvultra-embedder    DPO trainer    │
│  (RVF store, HTTP)   (candle-cuda, HTTP)  (trl + peft)   │
├──────────────────────────────────────────────────────────┤
│  brain.rvf           adapters/            profiles/      │
│  (cognitive store)   (LoRA weights)       (system TOML)  │
├──────────────────────────────────────────────────────────┤
│  Linux kernel + NVIDIA driver + systemd                  │
└──────────────────────────────────────────────────────────┘
```

## 10-Level Stack

| Level | Component | Status | Details |
|-------|-----------|--------|---------|
| 1 | Identity | Done | Ed25519 keys via `ruvultra-init identity` |
| 2 | Brain | Done | RVF cognitive container, 834+ memories |
| 3 | Embedder | Done | CUDA bge-small-en-v1.5, 384-d, 2ms/embed |
| 4 | Semantic search | Done | Partitioned cosine index, 22ms |
| 5 | MCP tools | Done | 102 stdio + 22 brain = 124 total |
| 6 | System profiles | Done | 6 profiles, atomic apply/rollback, MPS pre-flight |
| 7 | Desktop app | Done | Tauri v2 + Svelte 5, gold neural theme |
| 8 | Contrastive data | Done | 3,265 preference pairs exported nightly |
| 9 | DPO training | Done | LoRA adapter trained on 205 pairs, loss 0.081 |
| 10 | Adapter deployment | Next | Apply trained adapter to inference |

## Packages

| Package | Arch | Size | Description |
|---------|------|------|-------------|
| `ruos-core` | amd64 | 4.0 MB | MCP server (102 tools), profile helper, brain backend (RVF), init tool |
| `ruos-core` | arm64 | 829 KB | Same (no GPU deps) for Pi 5, Jetson, Apple Silicon |
| `ruos-brain-base` | all | 359 KB | Pre-trained brain.rvf — 50 curated memories in RVF format |
| `ruos-embedder` | amd64 | 68 MB | CUDA embedding service (candle + bge-small-en-v1.5, 384-d vectors) |
| `ruos-desktop` | amd64 | 4.8 MB | Tauri desktop dashboard (gold neural theme) |

## Storage Format

ruOS uses **RVF (RuVector Format)** as the native brain storage:
- Append-only cognitive containers with per-segment XXH3-128 hash chains
- 8 segment types: Memory, Vector, Manifest, Metadata, Delta, Snapshot, Tombstone, Extension
- In-memory partitioned cosine index rebuilt on startup
- Ed25519 signing for provenance (via `ruvultra-init identity`)
- Portable: `cp brain.rvf /media/usb/` is a complete brain backup

## Self-Improvement Loop

ruOS closes the learning loop — the machine improves from its own corrections:

```
Brain memories ──→ Nightly export ──→ Preference pairs (JSONL)
                                           │
                                           ▼
                                    DPO training (trl + peft)
                                           │
                                           ▼
                                    LoRA adapter (18 MB)
                                           │
                                           ▼
                                    Improved inference
```

- **Export**: `export_pairs.py` extracts chosen/rejected pairs from brain votes and corrections
- **Train**: `train_dpo.py` fine-tunes TinyLlama-1.1B with LoRA (r=16, beta=0.1) using DPO
- **Result**: 100% eval accuracy, reward margin 13.96, loss 0.081 on 205 real preference pairs

## Install

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

## MCP Tools (102)

The MCP server exposes 102 tools over stdio JSON-RPC 2.0:

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
- See [ADR-SYS-0014](docs/adr/) for architecture details

## Links

- Platform repo: [cognitum-one/ruVultra](https://github.com/cognitum-one/ruVultra) (MCP server, desktop app, ADRs)
- RuView: [ruvnet/RuView](https://github.com/ruvnet/RuView) (WiFi sensing firmware)
- Cognitum: [cognitum.one](https://cognitum.one)

## License

MIT
