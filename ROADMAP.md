# Roadmap — muro

Ideas for future improvements. Nothing is committed — just brainstorming.

## Mode Assistant Vocal

Turn muro into a voice assistant: speak to Groq, get a spoken response via Audrey.

- **"assistant"** → switch to assistant mode
- **"dictée"** → back to dictation mode
- Multi-turn conversations stored in SQLite
- **"nouvelle conversation"** → fresh context
- **"discussions"** → list past conversations
- Response read aloud, optionally pasted

## Notes Vocales

- **"note [content]"** → save a voice note to SQLite
- **"mes notes"** → paste recent notes
- Simple and lightweight, no categories

## Interface

- Menu bar icon showing status (active, recording, Groq on/off)
- Native macOS notifications with transcribed text
- Settings UI instead of .env file

## Intelligence

- Conversation mode with memory (multi-turn context)
- Context-aware behavior based on active app (Mail → email tone, Slack → short messages)
- Auto language detection (French ↔ English switch)
- Web search via Groq browser_search tool

## Performance

- Streaming Whisper (real-time transcription while speaking)
- VAD (Voice Activity Detection) to auto-trim silence
- Local LLM fallback (Ollama) when offline
- Cache repeated translations

## Cross-platform

- Windows port (replace CGEventTap → SetWindowsHookEx, AppleScript → SendInput)
- Linux port (replace CGEventTap → libinput, AppleScript → xdotool)
- cpal and Groq API already work cross-platform

## Quality of Life

- Custom voice commands via config file
- Configurable trigger key (not just Right Option)
- Rate limiting awareness (warn when approaching Groq quota)
- Encrypted SQLite history (sqlcipher)
- Audio recording export (save dictation as .wav)
