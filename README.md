# ruOS

The operating system for AI workstations. Local-first, contrastive-learning-native, runs on amd64 + arm64.

## Packages

| Package | Arch | Size | Description |
|---|---|---|---|
| `ruos-core` | amd64 | 4.0 MB | MCP server (102 tools), profile helper, brain backend (RVF), init tool |
| `ruos-core` | arm64 | 829 KB | MCP server, profile helper, init tool (no GPU deps) |
| `ruos-brain-base` | all | 359 KB | Pre-trained brain.rvf — 50 curated memories in RVF cognitive container format |
| `ruos-embedder` | amd64 | 68 MB | CUDA embedding service (candle + bge-small-en-v1.5, 384-d vectors) |
| `ruos-desktop` | amd64 | 4.8 MB | Optional Tauri desktop app (gold neural theme dashboard) |

## Storage format

ruOS uses **RVF (RuVector Format)** as the native brain storage:
- Append-only cognitive containers with per-segment XXH3-128 hash chains
- In-memory partitioned cosine index rebuilt on startup
- Ed25519 signing for provenance (via `ruvultra-init identity`)
- Portable: `cp brain.rvf /media/usb/` is a complete brain backup

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
docker compose up -d                    # CPU-only
docker compose -f docker-compose.gpu.yml up -d  # with NVIDIA GPU
```

### USB (offline, bootable)
```bash
# Create bootable USB:
sudo bash scripts/create-bootable-usb.sh /dev/sdX

# Or use the installer:
bash /media/usb/ruos-installer/install.sh
```

## Build from source

```bash
make all          # build amd64 + arm64 binaries
make deb          # package .debs
make deb-brain    # package pre-trained brain (RVF)
make deb-embedder # package CUDA embedder + model
make deb-desktop  # package Tauri desktop app
make test-docker  # test amd64 install
make test-arm64   # test arm64 via QEMU emulation
make release      # build + package + test everything
```

## Tested architectures

| Arch | Tier | Status | Devices |
|---|---|---|---|
| amd64 | 1 (CI) | 1,333 MCP calls/sec, <1ms cold start | Workstations, servers |
| arm64 | 1 (CI) | Tested via QEMU, 27 calls/sec emulated | Pi 5, Jetson, Apple Silicon |
| armv7 | 2 (community) | Base image boots, brain.rvf portable | Pi 3/4 (32-bit) |
| riscv64 | 2 (community) | Base image boots | StarFive, Sipeed |

## Links

- Platform repo: [cognitum-one/ruVultra](https://github.com/cognitum-one/ruVultra)
- Desktop app source: `cognitum-one/ruVultra` `/app/`
- ADRs: 13 architecture decision records in the platform repo

## License

MIT
