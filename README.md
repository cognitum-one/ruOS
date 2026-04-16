# ruOS

Cross-compile and `.deb` packaging pipeline for the ruvultra AI workstation.

**Targets:** amd64 (native x86-64) + arm64 (Pi/Jetson cross-compile)

## Packages

| Package | Arch | Size | Contents |
|---------|------|------|----------|
| `ruos-core` | amd64 | ~4 MB | ruvultra-mcp, ruvultra-profile, mcp-brain-server-local, mcp-brain, ruvultra-init, profiles, systemd units |
| `ruos-core` | arm64 | ~850 KB | ruvultra-mcp, ruvultra-profile, ruvultra-init, profiles, systemd units |
| `ruos-brain-base` | all | ~360 KB | Pre-seeded brain.rvf with 50 curated memories |
| `ruos-embedder` | amd64 | ~67 MB | ruos-embedder binary + bge-small-en-v1.5 model weights |

## Components

| Binary | Description | Arch |
|--------|-------------|------|
| `ruvultra-mcp` | 99-tool MCP server (JSON-RPC 2.0 over stdio) | amd64, arm64 |
| `ruvultra-profile` | System profile helper (sysctl, GPU, scheduler) | amd64, arm64 |
| `ruvultra-init` | Hardware detection and system initialization | amd64, arm64 |
| `mcp-brain-server-local` | Local brain backend (SQLite + HNSW, port 9876) | amd64 only |
| `mcp-brain` | 22-tool stdio MCP wrapper for brain operations | amd64 only |
| `ruos-embedder` | Local GPU embedder (bge-small-en-v1.5, CUDA) | amd64 only |

## Quick Start

### Full workstation (amd64 with GPU)

```bash
sudo dpkg -i ruos-core_0.7.0_amd64.deb
sudo dpkg -i ruos-brain-base_0.7.0_all.deb
sudo dpkg -i ruos-embedder_0.7.0_amd64.deb
sudo apt-get install -f
```

### Headless server (amd64, no GPU embedder)

```bash
sudo dpkg -i ruos-core_0.7.0_amd64.deb
sudo dpkg -i ruos-brain-base_0.7.0_all.deb
sudo apt-get install -f
```

### Raspberry Pi / Jetson (arm64)

```bash
sudo dpkg -i ruos-core_0.7.0_arm64.deb
sudo apt-get install -f
```

Note: arm64 does not include brain-server or mcp-brain binaries. Use a remote brain endpoint or install the brain-base package for offline reference.

## Build

```bash
make amd64          # native build
make arm64          # cross-compile via cross
make deb            # package both arch .debs
make deb-brain      # package base brain (requires running brain server)
make deb-embedder   # package embedder (amd64 only)
make release        # build everything + test
```

## Test

```bash
make test-docker    # test amd64 install in Docker
make test-arm64     # test arm64 install via QEMU emulation
```

## Brain Base Package

The `ruos-brain-base` package contains 50 curated memories across five categories:

- **architecture** (10): MCP server, brain backend, embedder, profiles, RVF format, contrastive learning
- **operations** (10): GPU health, service management, backups, drift detection, training export
- **troubleshooting** (10): brain offline, CUDA errors, profile failures, systemd issues
- **optimization** (10): GPU persistence, BBR+fq, zram, THP, WAL checkpoint, clock locking
- **contrastive** (10): DPO vs RLHF, preference pairs, hard negatives, InfoNCE, MicroLoRA

On install, the base brain is copied to `~/brain-data/brain.rvf` if no existing brain is found.

## ADRs

- ADR-SYS-0012: Cross-compile pipeline
- ADR-SYS-0007: Linux system profiles
- ADR-SYS-0002: Local private brain backend
- ADR-SYS-0006: Local vector store and embeddings
