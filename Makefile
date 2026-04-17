SHELL := /bin/bash
VERSION := 0.7.0
CRATES := ruvultra-mcp ruvultra-profile
CRATE_BASE := /home/ruvultra/projects/ruvultra-cognitum/crates
OUTDIR := /home/ruvultra/projects/ruVultra-linux/out
SCRIPTS := /home/ruvultra/projects/ruVultra-linux/scripts

.PHONY: all amd64 arm64 deb deb-amd64 deb-arm64 deb-brain deb-embedder clean test-docker test-arm64 release

all: amd64 arm64

amd64:
	@mkdir -p $(OUTDIR)/amd64
	@for crate in $(CRATES); do \
		echo "==> Building $$crate (amd64, native)"; \
		cd $(CRATE_BASE)/$$crate && \
		RUSTFLAGS="-C target-cpu=native" cargo build --release && \
		cp target/release/$$crate $(OUTDIR)/amd64/ && \
		echo "    -> $(OUTDIR)/amd64/$$crate ($$(stat --printf='%s' $(OUTDIR)/amd64/$$crate) bytes)"; \
	done

arm64:
	@mkdir -p $(OUTDIR)/arm64
	@for crate in $(CRATES); do \
		echo "==> Building $$crate (arm64, cross)"; \
		cd $(CRATE_BASE)/$$crate && \
		RUSTFLAGS="" CROSS_CONFIG="" cross build --release --target aarch64-unknown-linux-gnu && \
		cp target/aarch64-unknown-linux-gnu/release/$$crate $(OUTDIR)/arm64/ && \
		echo "    -> $(OUTDIR)/arm64/$$crate"; \
	done

deb-amd64: amd64
	$(SCRIPTS)/build-deb.sh amd64

deb-arm64: arm64
	$(SCRIPTS)/build-deb.sh arm64

deb: deb-amd64 deb-arm64

deb-brain:
	$(SCRIPTS)/build-brain-deb.sh

deb-desktop:
	$(SCRIPTS)/build-desktop-deb.sh

deb-embedder:
	$(SCRIPTS)/build-embedder-deb.sh

test-docker: deb-amd64 deb-brain
	$(SCRIPTS)/test-install.sh

test-arm64: deb-arm64
	$(SCRIPTS)/test-arm64-qemu.sh

release: deb deb-brain deb-embedder test-docker test-arm64

clean:
	rm -rf $(OUTDIR)
