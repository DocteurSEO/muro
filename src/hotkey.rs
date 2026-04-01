use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::mpsc::Sender;

use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    KeyPressed,
    KeyReleased,
}

// --- CoreGraphics FFI ---

type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;

const K_CG_HID_EVENT_TAP: u32 = 0;
const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
const K_CG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;
const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;
const K_CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x00080000;
const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFFFFFE;
const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;
const RIGHT_OPTION_KEYCODE: i64 = 0x3D; // 61

type CGEventTapCallBack = extern "C" fn(
    proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> *mut c_void;

    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *mut c_void,
        order: i64,
    ) -> *mut c_void;

    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();

    static kCFRunLoopCommonModes: *const c_void;
}

// --- State ---

static KEY_WAS_PRESSED: AtomicBool = AtomicBool::new(false);
static TAP_PORT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

extern "C" fn event_callback(
    _proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT {
        let tap = TAP_PORT.load(Ordering::SeqCst);
        if !tap.is_null() {
            unsafe { CGEventTapEnable(tap, true) };
        }
        return event;
    }

    if event_type != K_CG_EVENT_FLAGS_CHANGED {
        return event;
    }

    let flags = unsafe { CGEventGetFlags(event) };
    let keycode = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) };

    // Seulement la touche Option droite (keycode 0x3D)
    if keycode != RIGHT_OPTION_KEYCODE {
        return event;
    }

    let key_is_pressed = (flags & K_CG_EVENT_FLAG_MASK_ALTERNATE) != 0;
    let key_was_pressed = KEY_WAS_PRESSED.load(Ordering::SeqCst);

    if key_is_pressed != key_was_pressed {
        KEY_WAS_PRESSED.store(key_is_pressed, Ordering::SeqCst);
        let tx = unsafe { &*(user_info as *const Sender<HotkeyEvent>) };
        let ev = if key_is_pressed {
            HotkeyEvent::KeyPressed
        } else {
            HotkeyEvent::KeyReleased
        };
        let _ = tx.send(ev);
    }

    event
}

pub fn start_listening(tx: Sender<HotkeyEvent>) -> Result<()> {
    let tx_ptr = Box::into_raw(Box::new(tx)) as *mut c_void;
    let event_mask: u64 = 1 << K_CG_EVENT_FLAGS_CHANGED;

    let tap = unsafe {
        CGEventTapCreate(
            K_CG_HID_EVENT_TAP,
            K_CG_HEAD_INSERT_EVENT_TAP,
            K_CG_EVENT_TAP_OPTION_LISTEN_ONLY,
            event_mask,
            event_callback,
            tx_ptr,
        )
    };

    if tap.is_null() {
        bail!(
            "Impossible de creer le CGEventTap.\n\
             Va dans Reglages > Confidentialite > Accessibilite\n\
             et Reglages > Confidentialite > Surveillance de l'entree\n\
             pour autoriser le terminal."
        );
    }

    TAP_PORT.store(tap, Ordering::SeqCst);

    unsafe {
        let source = CFMachPortCreateRunLoopSource(ptr::null(), tap, 0);
        if source.is_null() {
            bail!("Impossible de creer la source CFRunLoop");
        }

        let run_loop = CFRunLoopGetCurrent();
        CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);

        tracing::info!("CGEventTap actif, en attente de Option droite (⌥)...");
        CFRunLoopRun();
    }

    Ok(())
}
