# ruOS — Agentic Workstation

This machine runs ruOS, the first agentic operating system built for Claude Code.

## First thing to do

Call `ruv_meta_boot_context` — it returns everything about this machine in one call:
GPU state, CPU, RAM, services, brain stats, profile, kernel tunings.

## Available tools

### ruvultra-mcp (102 tools)
System awareness: GPU, CUDA, CPU, memory, services, storage, network, docker, git, profiles.
Brain bridges: search, write, vote, list, checkpoint, export, workload.
Profile management: list, show, current, diff, validate, apply, rollback, history.

### mcp-brain (22 tools)
Knowledge: brain_share, brain_search, brain_vote, brain_list, brain_get, brain_delete.
Analysis: brain_drift, brain_partition, brain_transfer, brain_status, brain_sync.
Pages: brain_page_create, brain_page_get, brain_page_delta, brain_page_evidence, brain_page_promote.
Nodes: brain_node_publish, brain_node_list, brain_node_get, brain_node_revoke.

## How to use the brain

Every correction the user makes should become a preference pair:
```
ruv_brain_write_memory -> store what you learned
ruv_brain_write_preference_pair -> record "A was better than B"
ruv_brain_search -> find relevant past knowledge before answering
```

## Storage

The brain uses RVF (RuVector Format) — append-only cognitive containers.
Brain data: ~/brain-data/brain.rvf
Identity: ~/.config/ruvultra/identity.key (ed25519)

## System profiles

Switch the machine's posture:
- `gpu-train`: max GPU throughput for training
- `gpu-infer`: MPS enabled for contrastive decoding
- `cpu-bulk`: GPU power down, CPU-focused
- `interactive`: low latency for coding
- `power-save`: overnight idle

Use `ruv_profile_apply` (requires --enable-mutations on the MCP server).
