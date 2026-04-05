# muro

Assistant vocal macOS — dictée, traduction et commandes vocales, directement depuis n'importe quelle app.

Maintiens **Option droite (⌥)**, parle, relâche : le texte apparaît. ~1.2s de latence.

## Comment ça marche

```
[Micro] → Groq Whisper API (large-v3-turbo) → Groq LLM (cleanup) → Cmd+V
              ~250ms                              ~300ms
```

- **Transcription** : Whisper large-v3-turbo via Groq API (cloud, ultra-rapide)
- **Post-traitement** : ponctuation, majuscules, acronymes via LLM
- **Fallback** : Whisper local (tiny) si pas de réseau
- **Feedback vocal** : voix Audrey (macOS TTS) pour les confirmations
- **Historique** : SQLite local, 50 dernières entrées

## Commandes vocales

| Commande | Action |
|---|---|
| *(parler normalement)* | Dicte et colle le texte |
| **"traduis en anglais"** | Traduit le texte sélectionné (anglais, arabe, espagnol...) |
| **"traduis en arabe bonjour"** | Traduit le texte dicté (si rien n'est sélectionné) |
| **"corrige [texte]"** | Corrige le texte dicté |
| **"améliore"** | Sélectionne tout, améliore via IA, remplace |
| **"sélectionne"** | Cmd+A |
| **"lis"** | Lit le texte sélectionné à voix haute (Audrey) |
| **"stop"** | Arrête la lecture vocale |
| **"historique"** | Colle les 10 dernières dictées |
| **"active Groq"** | Active le post-traitement IA |
| **"désactive Groq"** | Désactive le post-traitement (dictée brute, plus rapide) |

Les commandes sont combinables : *"traduis en anglais et lis"*

## Installation

### Prérequis

- macOS (Apple Silicon recommandé)
- Rust toolchain (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Clé API Groq gratuite → [console.groq.com](https://console.groq.com)
- Autoriser le terminal dans **Réglages > Confidentialité > Accessibilité** et **Surveillance de l'entrée**

### Setup

```bash
git clone https://github.com/YOUR_USER/muro.git
cd muro

# Configurer la clé API
cp .env.example .env
# Éditer .env avec ta clé Groq

# Installer (compile + télécharge le modèle + lance au démarrage)
chmod +x install.sh
./install.sh
```

### Lancement manuel

```bash
./run.sh          # modèle tiny (fallback local)
./run.sh small    # meilleur fallback local
./run.sh medium   # encore mieux en local
```

## Architecture

```
src/
├── main.rs          # Boucle principale, commandes vocales, orchestration
├── hotkey.rs        # CGEventTap — détection Option droite
├── audio.rs         # Enregistrement micro (cpal, 16kHz mono)
├── groq.rs          # API Groq : Whisper (transcription) + LLM (cleanup/traduction)
├── transcriber.rs   # Whisper local (fallback, whisper.cpp via Metal)
├── paste.rs         # Simulation clavier AppleScript (Cmd+A/C/V)
└── history.rs       # SQLite — historique des dictées
```

### Flux de données

```
Option droite enfoncée
  → Enregistrement audio (cpal, PCM f32 16kHz)

Option droite relâchée
  → Audio envoyé à Groq Whisper API (~250ms)
  → Si échec réseau → fallback Whisper local
  → Détection de commande vocale (Rust, premier mot)
  → Si dictée normale → Groq LLM cleanup (~300ms)
  → Texte collé via Cmd+V (AppleScript)
```

### Rotation des clés API

Plusieurs clés Groq peuvent être fournies (séparées par des virgules dans `GROQ_API_KEYS`). Elles tournent en round-robin pour rester dans les limites du tier gratuit.

## Compatibilité

- **macOS** : natif (CoreGraphics, Metal, AppleScript)
- **Windows/Linux** : non compatible (dépend de CGEventTap, AppleScript, CoreAudio)

Un port nécessiterait de remplacer :
- `CGEventTap` → hook clavier global (Windows: `SetWindowsHookEx`, Linux: `libinput`)
- `AppleScript keystroke` → simulation clavier (`SendInput` sur Windows, `xdotool` sur Linux)
- `cpal` fonctionne déjà cross-platform
- L'API Groq est cross-platform

## Pour les contributeurs IA

> Ce projet a été construit avec Claude Code. Si tu es une IA qui améliore ce projet, voici le contexte :

### Décisions d'architecture
- **AppleScript pour TOUTES les simulations clavier** (Cmd+A/C/V) — CGEvent ne gère pas correctement les claviers AZERTY
- **Groq Cloud pour Whisper** plutôt que local — 250ms vs 4500ms, même qualité (large-v3-turbo)
- **Modèle local tiny comme fallback uniquement** — ne sert qu'en cas de panne réseau
- **Détection de commandes côté Rust** (premier mot) — ne PAS laisser le LLM décider, il interprète au lieu de corriger
- **Texte encadré par `---DEBUT/FIN TEXTE---`** dans les prompts cleanup — empêche le LLM de répondre aux questions dictées
- **`get_frontmost_app()` APRÈS le démarrage de l'enregistrement** — sinon latence perceptible au déclenchement

### Pièges connus
- Le LLM de cleanup a tendance à reformuler (changer "tu" en "vous", ajouter des mots). Le prompt doit être très strict
- Les keycodes CGEvent ne correspondent pas aux caractères sur AZERTY (keycode 0 = Q, pas A)
- `osascript` pour `get_frontmost_app` prend ~200ms — ne jamais bloquer l'enregistrement dessus
- Whisper peut confondre "dis-moi" avec "lis-moi" — la détection de "lis" doit être limitée aux 2 premiers mots

### Améliorations possibles
- Icône menu bar (statut : actif, enregistrement, Groq on/off)
- Notifications macOS natives avec le texte transcrit
- Mode conversation (poser une question, IA répond vocalement)
- Raccourcis custom via fichier de config
- Port Windows/Linux
- Streaming Whisper (transcription en temps réel pendant l'enregistrement)
- VAD (Voice Activity Detection) pour ignorer automatiquement le silence
