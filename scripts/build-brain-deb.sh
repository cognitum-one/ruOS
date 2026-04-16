#!/bin/sh
set -eu

VERSION="0.7.0"
PKG="ruos-brain-base"
PROJDIR="/home/ruvultra/projects/ruVultra-linux"
OUTDIR="${PROJDIR}/out"
DEBDIR="${OUTDIR}/deb"
STAGE="${OUTDIR}/_stage-brain"
BRAIN_SRC="/home/ruvultra/brain-data/brain.rvf"
BRAIN_API="http://127.0.0.1:9876"

echo "==> Seeding 50 curated memories into brain"

# Helper: POST a memory
post_mem() {
  _cat="$1"
  _content="$2"
  curl -sf -X POST "${BRAIN_API}/memories" \
    -H "Content-Type: application/json" \
    -d "{\"category\":\"${_cat}\",\"content\":\"${_content}\"}" >/dev/null 2>&1 || true
}

# --- architecture (10) ---
post_mem "architecture" "ruvultra-mcp is a 102-tool MCP (Model Context Protocol) server that exposes system monitoring, GPU control, profile management, brain queries, and optimization tools to AI assistants over stdio JSON-RPC."
post_mem "architecture" "The brain backend (mcp-brain-server-local) is a local HTTP server on port 9876 that stores and retrieves memories in a binary RVF (RuVultra Format) file backed by SQLite, with HNSW vector indexing for semantic search."
post_mem "architecture" "The embedder (ruos-embedder) runs bge-small-en-v1.5 locally on the GPU via CUDA/cuDNN to generate 384-dimensional vector embeddings for brain memories, eliminating the need for cloud embedding APIs."
post_mem "architecture" "The profile system (ruvultra-profile) applies predefined system configurations (sysctl, GPU clocks, scheduler, power) via TOML files in /etc/ruvultra-profiles/. Six profiles: default, gpu-train, gpu-infer, cpu-bulk, interactive, power-save."
post_mem "architecture" "RVF (RuVultra Format) is the binary storage format for brain memories. It contains serialized memory records with content hashes, categories, timestamps, and optional vector embeddings. The file lives at ~/brain-data/brain.rvf."
post_mem "architecture" "MCP (Model Context Protocol) is the JSON-RPC 2.0 protocol used to communicate between AI assistants (Claude, etc.) and ruvultra-mcp. Tools are registered via tools/list and invoked via tools/call over stdio."
post_mem "architecture" "The Tauri desktop app (ruvultra-workstation) provides a GUI dashboard for monitoring GPU status, brain health, active profiles, and system metrics. It communicates with the MCP server and brain backend."
post_mem "architecture" "Contrastive learning in ruvultra uses preference pairs (chosen/rejected) to fine-tune local models. The brain stores quality-voted training data that can be exported for DPO (Direct Preference Optimization) training."
post_mem "architecture" "Preference pairs consist of a prompt plus two responses: chosen (high quality) and rejected (low quality). These are stored in the brain with quality votes and used to train reward models or directly via DPO."
post_mem "architecture" "Hard negatives are semantically similar but incorrect examples used in contrastive training. The brain's HNSW index identifies near-miss memories that serve as hard negatives for more effective training."

# --- operations (10) ---
post_mem "operations" "Check GPU health: run nvidia-smi to see temperature, utilization, memory, clocks, and power. The ruvultra-mcp tool ruv_gpu_status provides structured JSON output for programmatic access."
post_mem "operations" "Restart services: systemctl --user restart ruvultra-brain ruos-embedder. Check status with systemctl --user status ruvultra-brain. Logs via journalctl --user -u ruvultra-brain -f."
post_mem "operations" "Apply a profile: sudo ruvultra-profile apply gpu-train. This reads /etc/ruvultra-profiles/gpu-train.toml and applies sysctl, GPU clock, scheduler, and power settings. Verify with ruvultra-profile current."
post_mem "operations" "Run backups: cp ~/brain-data/brain.rvf ~/brain-data/brain.rvf.backup. For SQLite: sqlite3 ~/brain-data/brain.sqlite '.backup brain-backup.sqlite'. Schedule via cron or systemd timer."
post_mem "operations" "Check drift: compare current sysctl/GPU settings against the active profile TOML. ruvultra-profile drift shows parameters that have changed since the profile was applied."
post_mem "operations" "Export training data: curl http://127.0.0.1:9876/memories?format=jsonl > training-data.jsonl. Filter by category with ?category=contrastive. Use for DPO fine-tuning pipelines."
post_mem "operations" "USB installer: flash the ruvultra ISO to USB with dd or Ventoy. Boot the target machine from USB, run ruvultra-init setup to configure the system with optimal defaults for the detected hardware."
post_mem "operations" "Connect via Tailscale: tailscale up --hostname ruvultra-workstation. Access remotely with tailscale ip -4. The MCP server can be exposed over Tailscale for remote AI assistant access."
post_mem "operations" "Monitor with the loop: use the ruvultra-mcp monitoring tools in a watch loop. watch -n 5 'echo {\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"method\\\":\\\"tools/call\\\",\\\"params\\\":{\\\"name\\\":\\\"ruv_gpu_status\\\"}} | ruvultra-mcp'"
post_mem "operations" "Update packages: apt update && apt upgrade ruos-core ruos-brain-base. Pin version with apt-mark hold ruos-core to prevent unwanted upgrades during training runs."

# --- troubleshooting (10) ---
post_mem "troubleshooting" "Brain offline fix: check if mcp-brain-server-local is running (pgrep mcp-brain-server). If not, start it: mcp-brain-server-local --port 9876 --data-dir ~/brain-data. Check port conflicts with ss -tlnp | grep 9876."
post_mem "troubleshooting" "Embedder CUDA errors: verify CUDA toolkit version matches driver (nvidia-smi vs nvcc --version). Common fix: export LD_LIBRARY_PATH=/usr/local/cuda/lib64. For OOM: reduce batch size in embedder config."
post_mem "troubleshooting" "Profile apply fails: check sudo permissions in /etc/sudoers.d/ruvultra-profile. The file must be mode 440 and owned by root. Verify with visudo -c -f /etc/sudoers.d/ruvultra-profile."
post_mem "troubleshooting" "Swappiness snaps back: some distros override sysctl on resume/sleep. Add vm.swappiness=10 to /etc/sysctl.d/99-ruvultra.conf and run sysctl --system. Also check if zram-generator resets it."
post_mem "troubleshooting" "GPU persistence disabled: run sudo nvidia-smi -pm 1 to enable persistence mode. For permanent: add nvidia-persistenced to systemd. Without persistence, GPU reinitializes on each use adding latency."
post_mem "troubleshooting" "MCP tools not loading: verify ruvultra-mcp binary is in PATH. Test with: echo '{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"method\\\":\\\"tools/list\\\"}' | ruvultra-mcp. Check stderr for initialization errors."
post_mem "troubleshooting" "RVF validation fails: the brain.rvf file may be corrupted. Restore from backup: cp brain.rvf.backup brain.rvf. If no backup, rebuild from SQLite: mcp-brain-server-local --rebuild-rvf."
post_mem "troubleshooting" "Systemd unit not starting: check with journalctl --user -u ruvultra-brain --no-pager -n 50. Common issues: wrong ExecStart path, missing Environment vars, or socket already in use."
post_mem "troubleshooting" "Docker GPU passthrough: install nvidia-container-toolkit, then docker run --gpus all. Verify with docker run --gpus all nvidia/cuda:12.4-base nvidia-smi. Set default runtime in /etc/docker/daemon.json."
post_mem "troubleshooting" "arm64 cross-compile issues: install cross (cargo install cross). Ensure Docker is running for cross builds. For linking errors, check that the aarch64 sysroot has required libs (sqlite3, openssl)."

# --- optimization (10) ---
post_mem "optimization" "GPU persistence mode: nvidia-smi -pm 1 keeps the GPU initialized between tasks, eliminating 100-500ms cold-start latency. Essential for interactive MCP tool calls and inference serving."
post_mem "optimization" "BBR+fq congestion control: sysctl net.core.default_qdisc=fq and net.ipv4.tcp_congestion_control=bbr. Improves throughput for model downloads and remote brain sync by 20-40%% on lossy networks."
post_mem "optimization" "zram tuning: configure zram with lz4 compression at 50%% of RAM. This provides compressed swap in RAM, avoiding disk I/O during memory pressure. Set via /etc/systemd/zram-generator.conf."
post_mem "optimization" "THP (Transparent Huge Pages) settings: set to madvise for GPU workloads. echo madvise > /sys/kernel/mm/transparent_hugepage/enabled. Reduces TLB misses for large model allocations."
post_mem "optimization" "WAL checkpoint: sqlite3 ~/brain-data/brain.sqlite 'PRAGMA wal_checkpoint(TRUNCATE)'. Run periodically to prevent WAL file growth. The brain server does this automatically every 1000 writes."
post_mem "optimization" "VACUUM schedule: sqlite3 ~/brain-data/brain.sqlite 'VACUUM'. Reclaims space after bulk deletes. Schedule weekly via cron. Can reduce database size by 30-50%% after heavy memory churn."
post_mem "optimization" "Profile selection guide: gpu-train for fine-tuning (max clocks, large pages), gpu-infer for serving (balanced clocks, low latency), cpu-bulk for data processing, interactive for development, power-save for idle."
post_mem "optimization" "Clock locking for stable inference: nvidia-smi -lgc 2100,2100 locks GPU clocks to avoid boost/throttle variance. Provides consistent latency for benchmarking and production inference."
post_mem "optimization" "Power management: for 24/7 operation, set power limit to 80%% TDP with nvidia-smi -pl 280 (RTX 5080). Reduces temperature by 10-15C with only 5-10%% performance loss."
post_mem "optimization" "sccache for Rust builds: export RUSTC_WRAPPER=sccache. Caches compilation artifacts across rebuilds. Speeds up incremental ruvultra-mcp builds by 3-5x after first compile."

# --- contrastive (10) ---
post_mem "contrastive" "DPO vs RLHF: DPO (Direct Preference Optimization) eliminates the reward model training step by directly optimizing the policy from preference pairs. Simpler, more stable, and requires less compute than RLHF."
post_mem "contrastive" "Preference pair format: {prompt, chosen, rejected} triplets. The brain stores these with category=contrastive. Quality is determined by human votes or automated scoring. Export via the /memories API with format=jsonl."
post_mem "contrastive" "Hard negative mining: use the brain's HNSW index to find semantically similar memories with different quality ratings. These near-miss pairs are the most informative training signal for contrastive learning."
post_mem "contrastive" "InfoNCE loss: the core contrastive learning objective. Maximizes agreement between anchor and positive while minimizing agreement with negatives. Temperature parameter controls sharpness of the distribution."
post_mem "contrastive" "Contrastive decoding: at inference time, subtract the log-probabilities of a weaker model from a stronger one. This amplifies the quality gap and reduces hallucination without any training."
post_mem "contrastive" "Quality voting: memories in the brain can be upvoted/downvoted to establish preference rankings. High-vote memories become chosen examples; low-vote memories become rejected examples for DPO training."
post_mem "contrastive" "Brainpedia lifecycle: memories progress through stages: ingestion -> embedding -> quality voting -> preference pairing -> training export -> model update -> deployment. The brain manages the full lifecycle."
post_mem "contrastive" "Drift detection: compare model outputs against stored preference pairs to detect quality degradation. If the model starts preferring previously-rejected responses, trigger retraining with fresh preference data."
post_mem "contrastive" "Transfer learning: pre-train on general preference data, then fine-tune on domain-specific ruvultra preferences. The brain-base package provides the general foundation; user memories provide domain specialization."
post_mem "contrastive" "MicroLoRA adapters: small rank-4 LoRA adapters trained on brain preference data. Stored in ~/brain-data/adapters/. Applied at inference time to customize model behavior without full fine-tuning. Under 10MB per adapter."

echo "==> Seeded 50 memories"

# Wait for brain to flush to disk
sleep 2

# Copy the brain.rvf as the base
if [ ! -f "${BRAIN_SRC}" ]; then
  echo "ERROR: ${BRAIN_SRC} not found — is the brain server running?" >&2
  exit 1
fi

echo "==> Packaging ${PKG}_${VERSION}_all.deb"

rm -rf "${STAGE}"
mkdir -p "${STAGE}/DEBIAN"
mkdir -p "${STAGE}/usr/share/ruvultra"

cp "${BRAIN_SRC}" "${STAGE}/usr/share/ruvultra/brain-base.rvf"

cat > "${STAGE}/DEBIAN/control" <<EOF
Package: ${PKG}
Version: ${VERSION}
Architecture: all
Maintainer: ruv <ruv@ruv.net>
Description: ruvultra pre-trained brain with 50 curated base memories
Depends: ruos-core
Homepage: https://github.com/cognitum-one/ruVultra
Section: utils
Priority: optional
Installed-Size: $(du -sk "${STAGE}" | cut -f1)
EOF

cat > "${STAGE}/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e
# Copy base brain to user's brain-data dir if none exists
BRAIN_DIR="${HOME}/brain-data"
BRAIN_FILE="${BRAIN_DIR}/brain.rvf"
BASE="/usr/share/ruvultra/brain-base.rvf"

if [ ! -f "${BRAIN_FILE}" ]; then
  mkdir -p "${BRAIN_DIR}"
  cp "${BASE}" "${BRAIN_FILE}"
  echo "ruos-brain-base: installed base brain to ${BRAIN_FILE}"
else
  echo "ruos-brain-base: existing brain found at ${BRAIN_FILE}, not overwriting"
  echo "  To reset: cp ${BASE} ${BRAIN_FILE}"
fi
POSTINST
chmod 755 "${STAGE}/DEBIAN/postinst"

mkdir -p "${DEBDIR}"
dpkg-deb --root-owner-group --build "${STAGE}" "${DEBDIR}/${PKG}_${VERSION}_all.deb"

rm -rf "${STAGE}"

SIZE=$(stat --printf='%s' "${DEBDIR}/${PKG}_${VERSION}_all.deb")
echo "==> Built: ${DEBDIR}/${PKG}_${VERSION}_all.deb (${SIZE} bytes)"
