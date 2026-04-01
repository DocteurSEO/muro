use anyhow::Result;
use std::ffi::c_void;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: *const c_void,
        keycode: u16,
        keydown: bool,
    ) -> *mut c_void;
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
const V_KEYCODE: u16 = 9;  // touche 'v' (meme position QWERTY/AZERTY)

fn simulate_cmd_key(keycode: u16) {
    unsafe {
        let down = CGEventCreateKeyboardEvent(std::ptr::null(), keycode, true);
        CGEventSetFlags(down, K_CG_EVENT_FLAG_MASK_COMMAND);
        CGEventPost(0, down);
        CFRelease(down as *const c_void);

        std::thread::sleep(std::time::Duration::from_millis(20));

        let up = CGEventCreateKeyboardEvent(std::ptr::null(), keycode, false);
        CGEventSetFlags(up, K_CG_EVENT_FLAG_MASK_COMMAND);
        CGEventPost(0, up);
        CFRelease(up as *const c_void);
    }
}

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

    // Simuler Cmd+V
    simulate_cmd_key(V_KEYCODE);

    // Restaurer le presse-papiers precedent
    if let Some(prev) = previous {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let _ = clipboard.set_text(&prev);
    }

    Ok(())
}
