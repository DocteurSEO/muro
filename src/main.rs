mod audio;
mod groq;
mod history;
mod hotkey;
mod paste;
mod transcriber;

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use tracing::{error, info};

static GROQ_ENABLED: AtomicBool = AtomicBool::new(true);

// --- Sons et voix ---

fn play_sound(name: &str) {
    let path = format!("/System/Library/Sounds/{}", name);
    thread::spawn(move || {
        let _ = Command::new("afplay").arg(path).output();
    });
}

fn sounds_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("muro")
        .join("sounds")
}

fn best_french_voice() -> &'static str {
    if let Ok(output) = Command::new("say").args(["-v", "?"]).output() {
        let voices = String::from_utf8_lossy(&output.stdout);
        if voices.contains("Audrey (Premium)") {
            return "Audrey (Premium)";
        }
        if voices.contains("Audrey") {
            return "Audrey";
        }
    }
    "Sandy"
}

fn init_voice_cache() {
    let dir = sounds_dir();
    let _ = std::fs::create_dir_all(&dir);

    let voice = best_french_voice();
    info!("Voix TTS: {}", voice);

    let phrases = [
        ("groq_on.aiff", "Groq activé"),
        ("groq_off.aiff", "Groq désactivé"),
        ("selected.aiff", "Sélectionné"),
        ("improved.aiff", "Texte amélioré"),
        ("translated.aiff", "Traduit"),
        ("corrected.aiff", "Corrigé"),
    ];

    for (file, text) in phrases {
        let path = dir.join(file);
        if !path.exists() {
            let _ = Command::new("say")
                .args(["-v", voice, "-o", &path.to_string_lossy(), text])
                .output();
        }
    }
}

fn speak(filename: &str) {
    let path = sounds_dir().join(filename);
    thread::spawn(move || {
        let _ = Command::new("afplay").arg(path).output();
    });
}

fn read_aloud(text: &str) {
    let voice = best_french_voice().to_string();
    let text = text.to_string();
    thread::spawn(move || {
        let _ = Command::new("say").args(["-v", &voice, &text]).output();
    });
}

fn stop_reading() {
    let _ = Command::new("killall").arg("say").output();
}

fn get_frontmost_app() -> Option<String> {
    let output = Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to get bundle identifier of first application process whose frontmost is true"])
        .output()
        .ok()?;
    let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if bundle_id.is_empty() { None } else { Some(bundle_id) }
}

fn refocus_app(app: &Option<String>) {
    if let Some(bundle_id) = app {
        let _ = Command::new("osascript")
            .args(["-e", &format!("tell application id \"{}\" to activate", bundle_id)])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Detecte s'il y a du texte selectionne et le retourne
fn grab_selection() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let previous = clipboard.get_text().ok();

    // Vider le presse-papiers pour detecter si Cmd+C copie quelque chose
    let _ = clipboard.set_text("");
    drop(clipboard);

    std::thread::sleep(std::time::Duration::from_millis(50));
    let copied = paste::copy_selection().unwrap_or_default();

    // Restaurer le presse-papiers d'origine
    if let Ok(mut cb) = arboard::Clipboard::new() {
        if let Some(ref prev) = previous {
            let _ = cb.set_text(prev);
        }
    }

    if copied.is_empty() {
        None // Pas de selection
    } else {
        Some(copied)
    }
}

// --- Commandes vocales ---

enum VoiceCommand {
    ActivateGroq,
    DeactivateGroq,
    SelectAll,
    Stop,
    ReadAloud,
    History,
    TranslateSelection(String),
    Correct(String),
    Improve,
    Dictation(String),
}

struct ParsedCommand {
    command: VoiceCommand,
    read_aloud: bool,
}

fn clean_word(w: &str) -> String {
    w.trim_matches(|c: char| c.is_ascii_punctuation()).to_string()
}

fn parse_command(text: &str) -> ParsedCommand {
    let lower = text.to_lowercase();
    let words: Vec<String> = lower.split_whitespace().map(|w| clean_word(w)).collect();
    let first = words.first().map(|s| s.as_str()).unwrap_or("");

    // "lis" doit etre dans les 2 premiers mots pour eviter les faux positifs
    // (ex: "dis-moi" confondu avec "lis-moi" par Whisper)
    let first_two = &words[..words.len().min(2)];
    let read_aloud = first_two.iter().any(|w| matches!(w.as_str(), "lis" | "lire"))
        || matches!(first, "lecture");

    // "stop" / "arrête"
    if matches!(first, "stop" | "arrête" | "arrete" | "arrêter" | "arreter" | "tais-toi") {
        return ParsedCommand { command: VoiceCommand::Stop, read_aloud: false };
    }

    // "historique"
    if matches!(first, "historique" | "historiques" | "history") {
        return ParsedCommand { command: VoiceCommand::History, read_aloud: false };
    }

    // "desactive groq"
    if matches!(first, "désactive" | "desactive" | "désactiver" | "desactiver") {
        if words.iter().any(|w| matches!(w.as_str(), "groq" | "grok" | "groc")) {
            return ParsedCommand { command: VoiceCommand::DeactivateGroq, read_aloud: false };
        }
    }

    // "active groq"
    if matches!(first, "active" | "activer" | "actif") {
        if words.iter().any(|w| matches!(w.as_str(), "groq" | "grok" | "groc")) {
            return ParsedCommand { command: VoiceCommand::ActivateGroq, read_aloud: false };
        }
    }

    // "selectionne" — dans les 3 premiers mots
    let first_three = &words[..words.len().min(3)];
    if first_three.iter().any(|w| {
        matches!(w.as_str(), "sélectionne" | "selectionne" | "sélectionner" | "selectionner"
            | "sélection" | "selection" | "sélectionné" | "selectionné" | "sélectionnez" | "selectionnez")
    }) {
        return ParsedCommand { command: VoiceCommand::SelectAll, read_aloud: false };
    }

    // "traduis [en anglais]" — traduit le texte SELECTIONNE
    if matches!(first, "traduis" | "traduire" | "traduit") {
        let lang = extract_lang(&lower, &["traduis", "traduire", "traduit"]);
        // "traduis ... et lis" → read_aloud meme si "lis" n'est pas dans les 2 premiers mots
        let has_lis_suffix = words.last().map(|w| w.as_str()) == Some("lis");
        return ParsedCommand { command: VoiceCommand::TranslateSelection(lang), read_aloud: read_aloud || has_lis_suffix };
    }

    // "corrige [texte dicte]"
    if matches!(first, "corrige" | "corriger" | "corriges") {
        let rest = extract_after(text, &["corrige", "corriger", "corriges"]);
        return ParsedCommand { command: VoiceCommand::Correct(rest), read_aloud };
    }

    // "ameliore" — selectionne + ameliore
    if matches!(first, "améliore" | "ameliore" | "améliorer" | "ameliorer") {
        return ParsedCommand { command: VoiceCommand::Improve, read_aloud };
    }

    // "lis" seul — lire le texte selectionne
    if read_aloud {
        return ParsedCommand { command: VoiceCommand::ReadAloud, read_aloud: true };
    }

    ParsedCommand { command: VoiceCommand::Dictation(text.to_string()), read_aloud: false }
}

/// Extrait l'instruction de langue apres le mot-cle (ex: "en anglais", "en espagnol")
fn extract_lang(lower: &str, keywords: &[&str]) -> String {
    for keyword in keywords {
        if let Some(pos) = lower.find(keyword) {
            let after = lower[pos + keyword.len()..].trim()
                .trim_matches(|c: char| c.is_ascii_punctuation())
                .trim_end_matches("et lis")
                .trim_end_matches("et lire")
                .trim();
            if !after.is_empty() {
                return after.to_string();
            }
        }
    }
    "en anglais".to_string() // defaut
}

fn extract_after(text: &str, keywords: &[&str]) -> String {
    let lower = text.to_lowercase();
    for keyword in keywords {
        if let Some(pos) = lower.find(keyword) {
            let after = text[pos + keyword.len()..].trim()
                .trim_matches(|c: char| c.is_ascii_punctuation())
                .trim_end_matches("et lis")
                .trim_end_matches("et lire")
                .trim();
            return after.to_string();
        }
    }
    String::new()
}

// --- Main ---

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let model_size = std::env::var("MURO_MODEL").unwrap_or_else(|_| "small".to_string());
    let model_file = format!("ggml-{}.bin", model_size);
    let models_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("muro")
        .join("models");
    let model_path = models_dir.join(&model_file);

    if !model_path.exists() {
        eprintln!("Modele Whisper '{}' introuvable: {}", model_size, model_path.display());
        eprintln!();
        eprintln!("Telecharge-le:");
        eprintln!("  mkdir -p {}", models_dir.display());
        eprintln!(
            "  curl -L -o {} https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            model_path.display(), model_file
        );
        std::process::exit(1);
    }

    if let Err(e) = history::init() {
        error!("Erreur init SQLite: {}", e);
    }

    info!("Initialisation du cache vocal...");
    init_voice_cache();

    info!("Chargement du modele Whisper ({})...", model_size);
    let whisper = transcriber::Transcriber::new(&model_path)?;
    info!("Modele charge!");

    let (tx, rx) = mpsc::channel::<hotkey::HotkeyEvent>();

    thread::spawn(move || {
        let mut recorder: Option<audio::Recorder> = None;
        let mut frontmost_app: Option<String> = None;

        loop {
            match rx.recv() {
                Ok(hotkey::HotkeyEvent::KeyPressed) => {
                    // Demarrer l'enregistrement IMMEDIATEMENT
                    play_sound("Tink.aiff");
                    match audio::Recorder::new() {
                        Ok(rec) => recorder = Some(rec),
                        Err(e) => error!("Erreur audio: {}", e),
                    }
                    // Recuperer l'app active en arriere-plan (pas urgent)
                    frontmost_app = get_frontmost_app();
                    info!("Enregistrement... (app: {:?})", frontmost_app);
                }
                Ok(hotkey::HotkeyEvent::KeyReleased) => {
                    if let Some(rec) = recorder.take() {
                        let audio_data = rec.stop();

                        let duration = audio_data.len() as f32 / 16000.0;
                        if duration < 0.5 {
                            info!("Trop court ({:.1}s), ignore", duration);
                            continue;
                        }

                        info!("Transcription ({:.1}s d'audio)...", duration);
                        let t0 = std::time::Instant::now();

                        // Essayer Groq Whisper API (cloud, rapide), fallback local
                        let transcription = groq::transcribe_audio(&audio_data)
                            .map(|t| { info!("Groq Whisper => cloud [{}ms]", t0.elapsed().as_millis()); t })
                            .or_else(|e| {
                                info!("Cloud indisponible ({}), fallback local...", e);
                                whisper.transcribe(&audio_data)
                            });

                        match transcription {
                            Ok(text) => {
                                let whisper_ms = t0.elapsed().as_millis();
                                if text.is_empty() {
                                    info!("Transcription vide");
                                    continue;
                                }

                                info!("Whisper => \"{}\" [{}ms]", text, whisper_ms);
                                let t1 = std::time::Instant::now();
                                refocus_app(&frontmost_app);
                                info!("Refocus [{}ms]", t1.elapsed().as_millis());

                                let parsed = parse_command(&text);

                                match parsed.command {
                                    VoiceCommand::Stop => {
                                        info!("Commande: stop lecture");
                                        stop_reading();
                                        history::save(&text, "", "stop");
                                    }
                                    VoiceCommand::History => {
                                        info!("Commande: historique");
                                        let hist = history::recent(10);
                                        info!("Historique:\n{}", hist);
                                        // Coller l'historique dans l'app active
                                        if let Err(e) = paste::paste_text(&hist) {
                                            error!("Erreur paste: {}", e);
                                        }
                                    }
                                    VoiceCommand::ActivateGroq => {
                                        GROQ_ENABLED.store(true, Ordering::SeqCst);
                                        info!("Groq ACTIVE");
                                        speak("groq_on.aiff");
                                        history::save(&text, "", "activate_groq");
                                    }
                                    VoiceCommand::DeactivateGroq => {
                                        GROQ_ENABLED.store(false, Ordering::SeqCst);
                                        info!("Groq DESACTIVE");
                                        speak("groq_off.aiff");
                                        history::save(&text, "", "deactivate_groq");
                                    }
                                    VoiceCommand::SelectAll => {
                                        info!("Commande: selectionner");
                                        paste::select_all();
                                        speak("selected.aiff");
                                        history::save(&text, "", "select_all");
                                    }
                                    VoiceCommand::ReadAloud => {
                                        info!("Commande: lire a voix haute");
                                        match grab_selection() {
                                            Some(selected) => {
                                                info!("Lecture de {} chars", selected.len());
                                                read_aloud(&selected);
                                                history::save(&text, &selected, "read_aloud");
                                            }
                                            None => info!("Rien de selectionne a lire"),
                                        }
                                    }
                                    VoiceCommand::TranslateSelection(lang) => {
                                        // Selection ? on traduit la selection
                                        // Pas de selection ? on traduit les mots dictes
                                        let to_translate = match grab_selection() {
                                            Some(selected) => {
                                                info!("Traduit selection: \"{}\" ({})", selected, lang);
                                                format!("{} : {}", lang, selected)
                                            }
                                            None => {
                                                info!("Traduit dicte: \"{}\"", lang);
                                                lang // contient deja "en anglais [texte dicte]"
                                            }
                                        };

                                        match groq::translate(&to_translate) {
                                            Ok(translated) => {
                                                info!("Traduit => \"{}\"", translated);
                                                speak("translated.aiff");
                                                if let Err(e) = paste::paste_text(&translated) {
                                                    error!("Erreur paste: {}", e);
                                                }
                                                if parsed.read_aloud {
                                                    std::thread::sleep(std::time::Duration::from_millis(800));
                                                    read_aloud(&translated);
                                                }
                                                history::save(&text, &translated, "translate");
                                            }
                                            Err(e) => {
                                                error!("Groq erreur: {}", e);
                                                play_sound("Basso.aiff");
                                            }
                                        }
                                    }
                                    VoiceCommand::Correct(rest) => {
                                        info!("Commande: corriger => \"{}\"", rest);
                                        match groq::correct(&rest) {
                                            Ok(corrected) => {
                                                info!("Corrige => \"{}\"", corrected);
                                                speak("corrected.aiff");
                                                if let Err(e) = paste::paste_text(&corrected) {
                                                    error!("Erreur paste: {}", e);
                                                }
                                                if parsed.read_aloud {
                                                    std::thread::sleep(std::time::Duration::from_millis(800));
                                                    read_aloud(&corrected);
                                                }
                                                history::save(&text, &corrected, "correct");
                                            }
                                            Err(e) => {
                                                error!("Groq erreur: {}", e);
                                                play_sound("Basso.aiff");
                                            }
                                        }
                                    }
                                    VoiceCommand::Improve => {
                                        info!("Commande: ameliorer le texte");
                                        paste::select_all();
                                        std::thread::sleep(std::time::Duration::from_millis(100));

                                        match paste::copy_selection() {
                                            Ok(selected) if !selected.is_empty() => {
                                                info!("Texte copie ({} chars)", selected.len());
                                                match groq::improve(&selected) {
                                                    Ok(improved) => {
                                                        info!("Ameliore => \"{}\"", improved);
                                                        speak("improved.aiff");
                                                        if let Err(e) = paste::paste_text(&improved) {
                                                            error!("Erreur paste: {}", e);
                                                        }
                                                        if parsed.read_aloud {
                                                            std::thread::sleep(std::time::Duration::from_millis(800));
                                                            read_aloud(&improved);
                                                        }
                                                        history::save(&text, &improved, "improve");
                                                    }
                                                    Err(e) => {
                                                        error!("Groq erreur: {}", e);
                                                        play_sound("Basso.aiff");
                                                    }
                                                }
                                            }
                                            Ok(_) => info!("Rien a ameliorer (texte vide)"),
                                            Err(e) => error!("Erreur copie: {}", e),
                                        }
                                    }
                                    VoiceCommand::Dictation(raw_text) => {
                                        let t2 = std::time::Instant::now();
                                        let final_text = if GROQ_ENABLED.load(Ordering::SeqCst) {
                                            match groq::cleanup(&raw_text) {
                                                Ok(cleaned) => {
                                                    info!("Groq   => \"{}\" [{}ms]", cleaned, t2.elapsed().as_millis());
                                                    cleaned
                                                }
                                                Err(e) => {
                                                    error!("Groq erreur: {}, texte brut", e);
                                                    raw_text.clone()
                                                }
                                            }
                                        } else {
                                            raw_text.clone()
                                        };

                                        play_sound("Pop.aiff");
                                        if let Err(e) = paste::paste_text(&final_text) {
                                            error!("Erreur paste: {}", e);
                                        }
                                        history::save(&raw_text, &final_text, "dictation");
                                    }
                                }
                            }
                            Err(e) => error!("Erreur transcription: {}", e),
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    info!("muro pret! Maintiens Option droite pour dicter.");
    info!("Commandes: lis, stop, traduis, corrige, ameliore, selectionne, active/desactive groq");
    hotkey::start_listening(tx)?;

    Ok(())
}
