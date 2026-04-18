#!/bin/bash
# ruOS Post-Install — run after Ubuntu install on Mac hardware
# Usage: curl -sL https://raw.githubusercontent.com/cognitum-one/ruOS/main/scripts/ruos-post-install.sh | sudo bash

set -e
echo "╔══════════════════════════════════════════════╗"
echo "║  ruOS Post-Install for Mac Hardware          ║"
echo "╚══════════════════════════════════════════════╝"
echo

# 1. Broadcom WiFi driver
echo "==> Installing Broadcom WiFi driver..."
apt-get update -qq
apt-get install -y bcmwl-kernel-source
modprobe -r brcmfmac brcmutil b43 bcma ssb 2>/dev/null
modprobe wl
echo "  WiFi driver: installed"

# 2. Blacklist conflicting modules
tee /etc/modprobe.d/ruos-broadcom.conf > /dev/null << 'EOF'
blacklist brcmfmac
blacklist brcmutil
blacklist b43
blacklist b43legacy
blacklist bcma
blacklist ssb
blacklist brcmsmac
EOF
echo "  Blacklist: set"

# 3. Enable WiFi
rfkill unblock all 2>/dev/null
nmcli radio wifi on 2>/dev/null
systemctl restart NetworkManager
sleep 3
echo "  WiFi radio: enabled"
echo ""
echo "  Available networks:"
nmcli device wifi list 2>/dev/null | head -10
echo ""

# 4. Install ruOS packages
echo "==> Installing ruOS..."
REPO="https://github.com/cognitum-one/ruOS/releases/latest/download"
ARCH=$(dpkg --print-architecture)
TMPDIR=$(mktemp -d)

for pkg in ruos-core_0.7.0_${ARCH}.deb ruos-brain-base_0.7.0_all.deb ruos-agent_1.1.0_all.deb ruos-embedder-intel_1.1.0_all.deb; do
    echo "  Downloading $pkg..."
    wget -q "$REPO/$pkg" -O "$TMPDIR/$pkg" 2>/dev/null && dpkg -i "$TMPDIR/$pkg" 2>/dev/null || echo "  (skipped $pkg)"
done
rm -rf "$TMPDIR"

# 5. Setup ruOS user environment
if [ -n "$SUDO_USER" ]; then
    USER_HOME=$(eval echo ~$SUDO_USER)
    su - $SUDO_USER -c '
        mkdir -p ~/.local/bin ~/brain-data ~/.config/systemd/user
        # Copy binaries
        for bin in /usr/local/bin/ruos-* /usr/local/bin/ruvultra-* /usr/local/bin/mcp-brain-*; do
            [ -f "$bin" ] && cp "$bin" ~/.local/bin/ 2>/dev/null
        done
        # Enable services
        systemctl --user daemon-reload 2>/dev/null
        systemctl --user enable --now ruvultra-brain.service 2>/dev/null
        systemctl --user enable --now ruos-agent.timer 2>/dev/null
        # Generate identity
        ~/.local/bin/ruvultra-init identity 2>/dev/null
    ' 2>/dev/null
    echo "  ruOS: configured for $SUDO_USER"
fi

echo ""
echo "╔══════════════════════════════════════════════╗"
echo "║  Done! WiFi + ruOS installed.                ║"
echo "║                                              ║"
echo "║  Connect WiFi:  nmcli device wifi connect    ║"
echo "║                 'SSID' password 'PASS'       ║"
echo "║  Check ruOS:    ruos-agent --status           ║"
echo "╚══════════════════════════════════════════════╝"
