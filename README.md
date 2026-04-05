# muro

> **Beta** — This project was built with AI (Claude Code) for personal use. It works well but is far from perfect. There are rough edges, occasional bugs, and plenty of room for improvement. Feel free to fork it, improve it, break it, rebuild it — it's yours.

macOS voice assistant — dictation, translation and voice commands, from any app.

Hold **Right Option**, speak, release: text appears. ~1.2s latency.

## How it works

```
[Mic] → Groq Whisper API (large-v3-turbo) → Groq LLM (cleanup) → Cmd+V
              ~250ms                              ~300ms
```

- **Transcription**: Whisper large-v3-turbo via Groq API (cloud, ultra-fast)
- **Post-processing**: punctuation, capitalization, acronyms via LLM
- **Fallback**: local Whisper (tiny) when offline
- **Voice feedback**: Audrey voice (macOS TTS) for confirmations
- **History**: local SQLite, last 50 entries

## Voice commands

| Command | Action |
|---|---|
| *(just speak)* | Dictate and paste text |
| **"traduis en anglais"** | Translate selected text (English, Arabic, Spanish...) |
| **"traduis en arabe bonjour"** | Translate dictated text (if nothing is selected) |
| **"corrige [text]"** | Correct dictated text |
| **"ameliore"** | Select all, improve via AI, replace |
| **"selectionne"** | Cmd+A |
| **"lis"** | Read selected text aloud (Audrey voice) |
| **"stop"** | Stop voice reading |
| **"historique"** | Paste the last 10 dictations |
| **"active Groq"** | Enable AI post-processing |
| **"desactive Groq"** | Disable post-processing (raw dictation, faster) |

Commands are composable: *"traduis en anglais et lis"* (translate and read aloud)

> Note: voice commands are in French. Adapting them to other languages requires modifying `parse_command()` in `main.rs`.

## Installation

### Prerequisites

- macOS (Apple Silicon recommended)
- Rust toolchain (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Free Groq API key → [console.groq.com](https://console.groq.com)
- Grant terminal access in **System Settings > Privacy > Accessibility** and **Input Monitoring**

### Setup

```bash
git clone https://github.com/YOUR_USER/muro.git
cd muro

# Configure your API key
cp .env.example .env
# Edit .env with your Groq key(s)

# Install (compiles + downloads model + launches at startup)
chmod +x install.sh
./install.sh
```

### Manual launch

```bash
./run.sh          # tiny model (local fallback)
./run.sh small    # better local fallback
./run.sh medium   # best local quality
```

## Architecture

```
src/
├── main.rs          # Main loop, voice commands, orchestration
├── hotkey.rs        # CGEventTap — Right Option key detection
├── audio.rs         # Mic recording (cpal, 16kHz mono)
├── groq.rs          # Groq API: Whisper (transcription) + LLM (cleanup/translation)
├── transcriber.rs   # Local Whisper fallback (whisper.cpp via Metal)
├── paste.rs         # Keyboard simulation via AppleScript (Cmd+A/C/V)
└── history.rs       # SQLite — dictation history
```

### Data flow

```
Right Option pressed
  → Start audio recording (cpal, PCM f32 16kHz)

Right Option released
  → Audio sent to Groq Whisper API (~250ms)
  → If network fails → fallback to local Whisper
  → Voice command detection (Rust, first word)
  → If normal dictation → Groq LLM cleanup (~300ms)
  → Text pasted via Cmd+V (AppleScript)
```

### API key rotation

Multiple Groq keys can be provided (comma-separated in `GROQ_API_KEYS`). They rotate round-robin to stay within the free tier limits.

## Compatibility

- **macOS**: native (CoreGraphics, Metal, AppleScript)
- **Windows/Linux**: not compatible (depends on CGEventTap, AppleScript, CoreAudio)

Porting would require replacing:
- `CGEventTap` → global keyboard hook (Windows: `SetWindowsHookEx`, Linux: `libinput`)
- `AppleScript keystroke` → keyboard simulation (`SendInput` on Windows, `xdotool` on Linux)
- `cpal` already works cross-platform
- Groq API is cross-platform

## Known issues & limitations

- Voice commands are French-only for now
- The LLM cleanup sometimes alters wording slightly (e.g. changing informal to formal tone) — the prompt is strict but not bulletproof
- Whisper may mishear command words (e.g. "dis-moi" → "lis-moi") — detection is limited to the first 2 words to reduce false positives
- macOS only — no Windows/Linux support
- No GUI — configuration via `.env` file only
- History is stored unencrypted in SQLite

## Contributing

This project is wide open for contributions. Some ideas:

- **Menu bar icon** — show status (active, recording, Groq on/off)
- **Native macOS notifications** with transcribed text
- **Conversation mode** — ask a question, AI answers vocally
- **Custom voice commands** via config file
- **Windows/Linux port**
- **Streaming Whisper** — real-time transcription while speaking
- **VAD** (Voice Activity Detection) to auto-ignore silence
- **Multilingual command support** — adapt commands to English, Spanish, etc.
- **Rate limiting** — detect when Groq quota is exhausted
- **Encrypted history** — SQLite with sqlcipher

## For AI contributors

> This project was built with Claude Code. If you're an AI improving this project, here's the context:

### Architecture decisions
- **AppleScript for ALL keyboard simulation** (Cmd+A/C/V) — CGEvent doesn't handle AZERTY keyboards correctly
- **Groq Cloud for Whisper** instead of local — 250ms vs 4500ms, same quality (large-v3-turbo)
- **Local tiny model as fallback only** — only used when network is down
- **Command detection in Rust** (first word) — do NOT let the LLM decide, it interprets instead of correcting
- **Text wrapped in `---BEGIN/END TEXT---`** markers in cleanup prompts — prevents the LLM from answering questions found in dictated text
- **`get_frontmost_app()` AFTER recording starts** — otherwise noticeable latency on key press

### Known pitfalls
- The cleanup LLM tends to rephrase (changing "tu" to "vous", adding words). The prompt must be very strict
- CGEvent keycodes don't match characters on AZERTY keyboards (keycode 0 = Q, not A)
- `osascript` for `get_frontmost_app` takes ~200ms — never block recording on it
- Whisper can confuse "dis-moi" with "lis-moi" — "lis" detection must be limited to the first 2 words

## License

MIT
