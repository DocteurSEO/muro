#!/bin/bash
set -e

echo "=== Installation de muro ==="

# Verifier .env
if [ ! -f .env ]; then
    echo "Copie .env.example → .env"
    cp .env.example .env
    echo "IMPORTANT: edite .env et ajoute ta cle API Groq"
    echo "  → https://console.groq.com"
    exit 1
fi

# Charger les cles
export $(grep -v '^#' .env | xargs)

# Compiler
echo "Compilation..."
export CXXFLAGS="-I/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/c++/v1"
cargo build --release

# Telecharger le modele Whisper tiny (fallback local)
MODELS_DIR="${HOME}/Library/Application Support/muro/models"
mkdir -p "$MODELS_DIR"
if [ ! -f "$MODELS_DIR/ggml-tiny.bin" ]; then
    echo "Telechargement du modele Whisper tiny (fallback)..."
    curl -L -o "$MODELS_DIR/ggml-tiny.bin" \
        https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin
fi

# Installer le launch agent
echo "Installation du launch agent..."
launchctl unload ~/Library/LaunchAgents/com.muro.agent.plist 2>/dev/null || true

# Generer le plist avec les cles depuis .env
cat > ~/Library/LaunchAgents/com.muro.agent.plist << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.muro.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>$(pwd)/target/release/muro</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>GROQ_API_KEYS</key>
        <string>${GROQ_API_KEYS}</string>
        <key>MURO_MODEL</key>
        <string>tiny</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/muro.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/muro.err</string>
</dict>
</plist>
PLIST

launchctl load ~/Library/LaunchAgents/com.muro.agent.plist

echo ""
echo "muro installe et demarre!"
echo "  - Option droite pour dicter"
echo "  - Commandes: traduis, corrige, ameliore, selectionne, lis, stop, historique"
echo "  - Logs: tail -f /tmp/muro.log"
echo "  - Desinstaller: launchctl unload ~/Library/LaunchAgents/com.muro.agent.plist"
