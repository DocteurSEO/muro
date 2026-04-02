use anyhow::Result;

fn applescript_keystroke(key: &str) {
    let script = format!(
        "tell application \"System Events\" to keystroke \"{}\" using command down",
        key
    );
    let _ = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output();
}

pub fn select_all() {
    applescript_keystroke("a");
}

pub fn copy_selection() -> Result<String> {
    applescript_keystroke("c");
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut clipboard = arboard::Clipboard::new()?;
    Ok(clipboard.get_text().unwrap_or_default())
}

pub fn paste_text(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;

    // Sauvegarder le presse-papiers actuel
    let previous = clipboard.get_text().ok();

    // Mettre le texte transcrit dans le presse-papiers
    clipboard.set_text(&format!(" {}", text))?;

    // Petit delai pour que le presse-papiers soit pret
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Cmd+V via AppleScript (compatible tous layouts)
    applescript_keystroke("v");

    // Restaurer le presse-papiers precedent
    if let Some(prev) = previous {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let _ = clipboard.set_text(&prev);
    }

    Ok(())
}
