#!/bin/bash
set -e

echo "=== Installation de muro ==="

# Compiler
echo "Compilation..."
export CXXFLAGS="-I/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/c++/v1"
cargo build --release

# Arreter l'ancien agent s'il tourne
launchctl unload ~/Library/LaunchAgents/com.muro.agent.plist 2>/dev/null || true

# Installer le launch agent
cp com.muro.agent.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.muro.agent.plist

echo ""
echo "muro installe et demarre!"
echo "  - Option droite (⌥) pour dicter"
echo "  - Commandes vocales: traduire, corriger, resumer..."
echo "  - Logs: tail -f /tmp/muro.log"
echo "  - Desinstaller: launchctl unload ~/Library/LaunchAgents/com.muro.agent.plist"
