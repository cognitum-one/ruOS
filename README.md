# ruVultra Linux Distribution Build Pipeline

Cross-compile and `.deb` packaging pipeline for the ruvultra Linux distribution.

**Targets:** amd64 (native) + arm64 (cross-compile for Pi/Jetson)

## Crates

| Crate | Description | Arch |
|-------|-------------|------|
| `ruvultra-mcp` | 99-tool MCP server | amd64, arm64 |
| `ruvultra-profile` | System profile helper | amd64, arm64 |
| `ruvultra-embedder` | CUDA embedder | amd64 only (GPU) |

## Quick Start

```bash
make amd64          # native build
make arm64          # cross-compile via `cross`
make deb            # build .deb packages for both archs
bash scripts/test-install.sh  # verify in Docker
```

## ADRs

- ADR-SYS-0012: Cross-compile pipeline
- ADR-SYS-0007: Linux system profiles
- ADR-SYS-0002: Local private brain backend
- ADR-SYS-0006: Local vector store and embeddings
