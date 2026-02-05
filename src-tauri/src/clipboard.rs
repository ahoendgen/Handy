use crate::input::{self, EnigoState};
use crate::settings::{get_settings, ClipboardHandling, PasteMethod, TriggerActionType, TriggerWord};
use enigo::{Enigo, Key, Keyboard};
use log::info;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[cfg(target_os = "linux")]
use crate::utils::{is_kde_wayland, is_wayland};
#[cfg(target_os = "linux")]
use std::process::Command;

/// Pastes text using the clipboard: saves current content, writes text, sends paste keystroke, restores clipboard.
fn paste_via_clipboard(
    enigo: &mut Enigo,
    text: &str,
    app_handle: &AppHandle,
    paste_method: &PasteMethod,
    paste_delay_ms: u64,
) -> Result<(), String> {
    let clipboard = app_handle.clipboard();
    let clipboard_content = clipboard.read_text().unwrap_or_default();

    // Write text to clipboard first
    // On Wayland, prefer wl-copy for better compatibility (especially with umlauts)
    #[cfg(target_os = "linux")]
    let write_result = if is_wayland() && is_wl_copy_available() {
        info!("Using wl-copy for clipboard write on Wayland");
        write_clipboard_via_wl_copy(text)
    } else {
        clipboard
            .write_text(text)
            .map_err(|e| format!("Failed to write to clipboard: {}", e))
    };

    #[cfg(not(target_os = "linux"))]
    let write_result = clipboard
        .write_text(text)
        .map_err(|e| format!("Failed to write to clipboard: {}", e));

    write_result?;

    std::thread::sleep(Duration::from_millis(paste_delay_ms));

    // Send paste key combo
    #[cfg(target_os = "linux")]
    let key_combo_sent = try_send_key_combo_linux(paste_method)?;

    #[cfg(not(target_os = "linux"))]
    let key_combo_sent = false;

    // Fall back to enigo if no native tool handled it
    if !key_combo_sent {
        match paste_method {
            PasteMethod::CtrlV => input::send_paste_ctrl_v(enigo)?,
            PasteMethod::CtrlShiftV => input::send_paste_ctrl_shift_v(enigo)?,
            PasteMethod::ShiftInsert => input::send_paste_shift_insert(enigo)?,
            _ => return Err("Invalid paste method for clipboard paste".into()),
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Restore original clipboard content
    // On Wayland, prefer wl-copy for better compatibility
    #[cfg(target_os = "linux")]
    if is_wayland() && is_wl_copy_available() {
        let _ = write_clipboard_via_wl_copy(&clipboard_content);
    } else {
        let _ = clipboard.write_text(&clipboard_content);
    }

    #[cfg(not(target_os = "linux"))]
    let _ = clipboard.write_text(&clipboard_content);

    Ok(())
}

/// Attempts to send a key combination using Linux-native tools.
/// Returns `Ok(true)` if a native tool handled it, `Ok(false)` to fall back to enigo.
#[cfg(target_os = "linux")]
fn try_send_key_combo_linux(paste_method: &PasteMethod) -> Result<bool, String> {
    if is_wayland() {
        // Wayland: prefer wtype (but not on KDE), then dotool, then ydotool
        // Note: wtype doesn't work on KDE (no zwp_virtual_keyboard_manager_v1 support)
        if !is_kde_wayland() && is_wtype_available() {
            info!("Using wtype for key combo");
            send_key_combo_via_wtype(paste_method)?;
            return Ok(true);
        }
        if is_dotool_available() {
            info!("Using dotool for key combo");
            send_key_combo_via_dotool(paste_method)?;
            return Ok(true);
        }
        if is_ydotool_available() {
            info!("Using ydotool for key combo");
            send_key_combo_via_ydotool(paste_method)?;
            return Ok(true);
        }
    } else {
        // X11: prefer xdotool, then ydotool
        if is_xdotool_available() {
            info!("Using xdotool for key combo");
            send_key_combo_via_xdotool(paste_method)?;
            return Ok(true);
        }
        if is_ydotool_available() {
            info!("Using ydotool for key combo");
            send_key_combo_via_ydotool(paste_method)?;
            return Ok(true);
        }
    }

    Ok(false)
}

/// Attempts to type text directly using Linux-native tools.
/// Returns `Ok(true)` if a native tool handled it, `Ok(false)` to fall back to enigo.
#[cfg(target_os = "linux")]
fn try_direct_typing_linux(text: &str) -> Result<bool, String> {
    if is_wayland() {
        // KDE Wayland: prefer kwtype (uses KDE Fake Input protocol, supports umlauts)
        if is_kde_wayland() && is_kwtype_available() {
            info!("Using kwtype for direct text input on KDE Wayland");
            type_text_via_kwtype(text)?;
            return Ok(true);
        }
        // Wayland: prefer wtype, then dotool, then ydotool
        // Note: wtype doesn't work on KDE (no zwp_virtual_keyboard_manager_v1 support)
        if !is_kde_wayland() && is_wtype_available() {
            info!("Using wtype for direct text input");
            type_text_via_wtype(text)?;
            return Ok(true);
        }
        if is_dotool_available() {
            info!("Using dotool for direct text input");
            type_text_via_dotool(text)?;
            return Ok(true);
        }
        if is_ydotool_available() {
            info!("Using ydotool for direct text input");
            type_text_via_ydotool(text)?;
            return Ok(true);
        }
    } else {
        // X11: prefer xdotool, then ydotool
        if is_xdotool_available() {
            info!("Using xdotool for direct text input");
            type_text_via_xdotool(text)?;
            return Ok(true);
        }
        if is_ydotool_available() {
            info!("Using ydotool for direct text input");
            type_text_via_ydotool(text)?;
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if wtype is available (Wayland text input tool)
#[cfg(target_os = "linux")]
fn is_wtype_available() -> bool {
    Command::new("which")
        .arg("wtype")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if dotool is available (another Wayland text input tool)
#[cfg(target_os = "linux")]
fn is_dotool_available() -> bool {
    Command::new("which")
        .arg("dotool")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if ydotool is available (uinput-based, works on both Wayland and X11)
#[cfg(target_os = "linux")]
fn is_ydotool_available() -> bool {
    Command::new("which")
        .arg("ydotool")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn is_xdotool_available() -> bool {
    Command::new("which")
        .arg("xdotool")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if kwtype is available (KDE Wayland virtual keyboard input tool)
#[cfg(target_os = "linux")]
fn is_kwtype_available() -> bool {
    Command::new("which")
        .arg("kwtype")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if wl-copy is available (Wayland clipboard tool)
#[cfg(target_os = "linux")]
fn is_wl_copy_available() -> bool {
    Command::new("which")
        .arg("wl-copy")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Type text directly via wtype on Wayland.
#[cfg(target_os = "linux")]
fn type_text_via_wtype(text: &str) -> Result<(), String> {
    let output = Command::new("wtype")
        .arg("--") // Protect against text starting with -
        .arg(text)
        .output()
        .map_err(|e| format!("Failed to execute wtype: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wtype failed: {}", stderr));
    }

    Ok(())
}

/// Type text directly via xdotool on X11.
#[cfg(target_os = "linux")]
fn type_text_via_xdotool(text: &str) -> Result<(), String> {
    let output = Command::new("xdotool")
        .arg("type")
        .arg("--clearmodifiers")
        .arg("--")
        .arg(text)
        .output()
        .map_err(|e| format!("Failed to execute xdotool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("xdotool failed: {}", stderr));
    }

    Ok(())
}

/// Type text directly via dotool (works on both Wayland and X11 via uinput).
#[cfg(target_os = "linux")]
fn type_text_via_dotool(text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("dotool")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn dotool: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        // dotool uses "type <text>" command
        writeln!(stdin, "type {}", text)
            .map_err(|e| format!("Failed to write to dotool stdin: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for dotool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("dotool failed: {}", stderr));
    }

    Ok(())
}

/// Type text directly via ydotool (uinput-based, requires ydotoold daemon).
#[cfg(target_os = "linux")]
fn type_text_via_ydotool(text: &str) -> Result<(), String> {
    let output = Command::new("ydotool")
        .arg("type")
        .arg("--")
        .arg(text)
        .output()
        .map_err(|e| format!("Failed to execute ydotool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ydotool failed: {}", stderr));
    }

    Ok(())
}

/// Type text directly via kwtype (KDE Wayland virtual keyboard, uses KDE Fake Input protocol).
#[cfg(target_os = "linux")]
fn type_text_via_kwtype(text: &str) -> Result<(), String> {
    let output = Command::new("kwtype")
        .arg("--")
        .arg(text)
        .output()
        .map_err(|e| format!("Failed to execute kwtype: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("kwtype failed: {}", stderr));
    }

    Ok(())
}

/// Write text to clipboard via wl-copy (Wayland clipboard tool).
#[cfg(target_os = "linux")]
fn write_clipboard_via_wl_copy(text: &str) -> Result<(), String> {
    let output = Command::new("wl-copy")
        .arg("--")
        .arg(text)
        .output()
        .map_err(|e| format!("Failed to execute wl-copy: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wl-copy failed: {}", stderr));
    }

    Ok(())
}

/// Send a key combination (e.g., Ctrl+V) via wtype on Wayland.
#[cfg(target_os = "linux")]
fn send_key_combo_via_wtype(paste_method: &PasteMethod) -> Result<(), String> {
    let args: Vec<&str> = match paste_method {
        PasteMethod::CtrlV => vec!["-M", "ctrl", "-k", "v"],
        PasteMethod::ShiftInsert => vec!["-M", "shift", "-k", "Insert"],
        PasteMethod::CtrlShiftV => vec!["-M", "ctrl", "-M", "shift", "-k", "v"],
        _ => return Err("Unsupported paste method".into()),
    };

    let output = Command::new("wtype")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute wtype: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wtype failed: {}", stderr));
    }

    Ok(())
}

/// Send a key combination (e.g., Ctrl+V) via dotool.
#[cfg(target_os = "linux")]
fn send_key_combo_via_dotool(paste_method: &PasteMethod) -> Result<(), String> {
    let command;
    match paste_method {
        PasteMethod::CtrlV => command = "echo key ctrl+v | dotool",
        PasteMethod::ShiftInsert => command = "echo key shift+insert | dotool",
        PasteMethod::CtrlShiftV => command = "echo key ctrl+shift+v | dotool",
        _ => return Err("Unsupported paste method".into()),
    }
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| format!("Failed to execute dotool: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("dotool failed: {}", stderr));
    }

    Ok(())
}

/// Send a key combination (e.g., Ctrl+V) via ydotool (requires ydotoold daemon).
#[cfg(target_os = "linux")]
fn send_key_combo_via_ydotool(paste_method: &PasteMethod) -> Result<(), String> {
    // ydotool uses Linux input event keycodes with format <keycode>:<pressed>
    // where pressed is 1 for down, 0 for up. Keycodes: ctrl=29, shift=42, v=47, insert=110
    let args: Vec<&str> = match paste_method {
        PasteMethod::CtrlV => vec!["key", "29:1", "47:1", "47:0", "29:0"],
        PasteMethod::ShiftInsert => vec!["key", "42:1", "110:1", "110:0", "42:0"],
        PasteMethod::CtrlShiftV => vec!["key", "29:1", "42:1", "47:1", "47:0", "42:0", "29:0"],
        _ => return Err("Unsupported paste method".into()),
    };

    let output = Command::new("ydotool")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute ydotool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ydotool failed: {}", stderr));
    }

    Ok(())
}

/// Send a key combination (e.g., Ctrl+V) via xdotool on X11.
#[cfg(target_os = "linux")]
fn send_key_combo_via_xdotool(paste_method: &PasteMethod) -> Result<(), String> {
    let key_combo = match paste_method {
        PasteMethod::CtrlV => "ctrl+v",
        PasteMethod::CtrlShiftV => "ctrl+shift+v",
        PasteMethod::ShiftInsert => "shift+Insert",
        _ => return Err("Unsupported paste method".into()),
    };

    let output = Command::new("xdotool")
        .arg("key")
        .arg("--clearmodifiers")
        .arg(key_combo)
        .output()
        .map_err(|e| format!("Failed to execute xdotool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("xdotool failed: {}", stderr));
    }

    Ok(())
}

// ============================================================================
// Trigger Word Processing
// ============================================================================

/// Represents a segment of output - either text to type or a key to press
#[derive(Debug, Clone)]
enum OutputSegment {
    Text(String),
    KeyPress(String),
}

/// Process text through trigger word replacement.
/// Returns segments to be output (text and key presses interleaved).
fn process_trigger_words(text: &str, trigger_words: &[TriggerWord]) -> Vec<OutputSegment> {
    // Build a list of enabled triggers, sorted by phrase length (longest first)
    // to ensure longer phrases match before shorter ones
    let mut triggers: Vec<_> = trigger_words
        .iter()
        .filter(|t| t.enabled)
        .collect();
    triggers.sort_by(|a, b| b.trigger_phrase.len().cmp(&a.trigger_phrase.len()));

    if triggers.is_empty() {
        return vec![OutputSegment::Text(text.to_string())];
    }

    let mut segments = Vec::new();
    let mut remaining = text.to_string();

    while !remaining.is_empty() {
        let mut found_match = false;
        let remaining_lower = remaining.to_lowercase();

        for trigger in &triggers {
            let phrase_lower = trigger.trigger_phrase.to_lowercase();

            // Find the trigger phrase with word boundary check
            if let Some(pos) = find_word_boundary_match(&remaining_lower, &phrase_lower) {
                // Add text before the trigger as a text segment
                if pos > 0 {
                    let before = &remaining[..pos];
                    // Trim trailing space before a trigger
                    let before_trimmed = before.trim_end();
                    if !before_trimmed.is_empty() {
                        segments.push(OutputSegment::Text(before_trimmed.to_string()));
                    }
                }

                // Add the trigger action
                match trigger.action_type {
                    TriggerActionType::TextReplacement => {
                        segments.push(OutputSegment::Text(trigger.action_value.clone()));
                    }
                    TriggerActionType::KeyPress => {
                        segments.push(OutputSegment::KeyPress(trigger.action_value.clone()));
                    }
                }

                // Continue with the text after the trigger
                let end_pos = pos + trigger.trigger_phrase.len();
                remaining = if end_pos < remaining.len() {
                    let after = &remaining[end_pos..];
                    // Skip trailing punctuation directly after a KeyPress trigger
                    // (e.g., "enter." -> the "." is added by speech recognition)
                    let after = if trigger.action_type == TriggerActionType::KeyPress {
                        after.trim_start_matches(|c: char| c.is_ascii_punctuation() && c != '\'')
                    } else {
                        after
                    };
                    // Skip any leading space after the trigger
                    after.trim_start().to_string()
                } else {
                    String::new()
                };

                found_match = true;
                break;
            }
        }

        if !found_match {
            // No trigger found - add remaining text and break
            segments.push(OutputSegment::Text(remaining));
            break;
        }
    }

    // Merge consecutive text segments
    merge_text_segments(segments)
}

/// Find a trigger phrase at a word boundary in the text.
/// Returns the position if found, None otherwise.
fn find_word_boundary_match(text: &str, phrase: &str) -> Option<usize> {
    let mut search_start = 0;

    while let Some(relative_pos) = text[search_start..].find(phrase) {
        let pos = search_start + relative_pos;
        let end_pos = pos + phrase.len();

        // Check if it's at a word boundary
        let at_start = pos == 0 || !text[..pos].chars().last().unwrap_or(' ').is_alphanumeric();
        let at_end = end_pos >= text.len()
            || !text[end_pos..].chars().next().unwrap_or(' ').is_alphanumeric();

        if at_start && at_end {
            return Some(pos);
        }

        // Continue searching after this position
        search_start = pos + 1;
        if search_start >= text.len() {
            break;
        }
    }

    None
}

/// Merge consecutive text segments into single segments.
fn merge_text_segments(segments: Vec<OutputSegment>) -> Vec<OutputSegment> {
    let mut merged = Vec::new();
    let mut current_text = String::new();

    for segment in segments {
        match segment {
            OutputSegment::Text(t) => {
                if !current_text.is_empty() {
                    current_text.push(' ');
                }
                current_text.push_str(&t);
            }
            OutputSegment::KeyPress(k) => {
                if !current_text.is_empty() {
                    merged.push(OutputSegment::Text(current_text));
                    current_text = String::new();
                }
                merged.push(OutputSegment::KeyPress(k));
            }
        }
    }

    if !current_text.is_empty() {
        merged.push(OutputSegment::Text(current_text));
    }

    merged
}

/// Send a key press using enigo.
fn send_key_press(enigo: &mut Enigo, key_name: &str) -> Result<(), String> {
    let key = match key_name.to_lowercase().as_str() {
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "escape" | "esc" => Key::Escape,
        "space" => Key::Space,
        "delete" | "del" => Key::Delete,
        "up" | "uparrow" => Key::UpArrow,
        "down" | "downarrow" => Key::DownArrow,
        "left" | "leftarrow" => Key::LeftArrow,
        "right" | "rightarrow" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        _ => return Err(format!("Unknown key: {}", key_name)),
    };

    // Use Press + Release instead of Click for better compatibility
    enigo
        .key(key, enigo::Direction::Press)
        .map_err(|e| format!("Failed to press key '{}': {}", key_name, e))?;

    std::thread::sleep(Duration::from_millis(10));

    enigo
        .key(key, enigo::Direction::Release)
        .map_err(|e| format!("Failed to release key '{}': {}", key_name, e))
}

/// Send a key press using Linux native tools (for Wayland/X11 compatibility).
#[cfg(target_os = "linux")]
fn send_key_press_linux(key_name: &str) -> Result<bool, String> {
    use crate::utils::{is_kde_wayland, is_wayland};

    let key_arg = match key_name.to_lowercase().as_str() {
        "enter" | "return" => "Return",
        "tab" => "Tab",
        "backspace" => "BackSpace",
        "escape" | "esc" => "Escape",
        "space" => "space",
        "delete" | "del" => "Delete",
        _ => return Ok(false), // Let enigo handle other keys
    };

    if is_wayland() {
        // On Wayland, use wtype (unless KDE), dotool, or ydotool
        if !is_kde_wayland() && is_wtype_available() {
            let output = std::process::Command::new("wtype")
                .arg("-k")
                .arg(key_arg)
                .output()
                .map_err(|e| format!("Failed to execute wtype: {}", e))?;

            if output.status.success() {
                return Ok(true);
            }
        }

        if is_dotool_available() {
            let command = format!("echo key {} | dotool", key_arg.to_lowercase());
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output()
                .map_err(|e| format!("Failed to execute dotool: {}", e))?;

            if output.status.success() {
                return Ok(true);
            }
        }
    } else if is_xdotool_available() {
        // On X11, use xdotool
        let output = std::process::Command::new("xdotool")
            .arg("key")
            .arg("--clearmodifiers")
            .arg(key_arg)
            .output()
            .map_err(|e| format!("Failed to execute xdotool: {}", e))?;

        if output.status.success() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Types text directly by simulating individual key presses.
fn paste_direct(enigo: &mut Enigo, text: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if try_direct_typing_linux(text)? {
            return Ok(());
        }
        info!("Falling back to enigo for direct text input");
    }

    input::paste_text_direct(enigo, text)
}

pub fn paste(text: String, app_handle: AppHandle) -> Result<(), String> {
    let settings = get_settings(&app_handle);
    let paste_method = settings.paste_method;
    let paste_delay_ms = settings.paste_delay_ms;

    // Append trailing space if setting is enabled
    let text = if settings.append_trailing_space {
        format!("{} ", text)
    } else {
        text
    };

    info!(
        "Using paste method: {:?}, delay: {}ms",
        paste_method, paste_delay_ms
    );

    // Get the managed Enigo instance
    let enigo_state = app_handle
        .try_state::<EnigoState>()
        .ok_or("Enigo state not initialized")?;
    let mut enigo = enigo_state
        .0
        .lock()
        .map_err(|e| format!("Failed to lock Enigo: {}", e))?;

    // Process trigger words if enabled
    if settings.trigger_words_enabled && !settings.trigger_words.is_empty() {
        let segments = process_trigger_words(&text, &settings.trigger_words);
        info!("Trigger words enabled, processing {} segments", segments.len());

        for segment in &segments {
            match segment {
                OutputSegment::Text(t) if !t.is_empty() => {
                    info!("Pasting text segment: '{}'", t);
                    paste_segment(&mut enigo, t, &app_handle, &paste_method, paste_delay_ms)?;
                    // Wait for paste to complete before processing next segment
                    std::thread::sleep(Duration::from_millis(100));
                }
                OutputSegment::KeyPress(key) => {
                    info!("Sending key press: '{}'", key);
                    // Try Linux native tools first, fall back to enigo
                    #[cfg(target_os = "linux")]
                    {
                        if !send_key_press_linux(key)? {
                            send_key_press(&mut enigo, key)?;
                        }
                    }
                    #[cfg(not(target_os = "linux"))]
                    send_key_press(&mut enigo, key)?;

                    // Delay after key press to let the application process it
                    std::thread::sleep(Duration::from_millis(50));
                }
                _ => {}
            }
        }
    } else {
        // No trigger words - use original behavior
        paste_segment(&mut enigo, &text, &app_handle, &paste_method, paste_delay_ms)?;
    }

    // After pasting, optionally copy to clipboard based on settings
    if settings.clipboard_handling == ClipboardHandling::CopyToClipboard {
        let clipboard = app_handle.clipboard();
        clipboard
            .write_text(&text)
            .map_err(|e| format!("Failed to copy to clipboard: {}", e))?;
    }

    Ok(())
}

/// Paste a single text segment using the configured method.
fn paste_segment(
    enigo: &mut Enigo,
    text: &str,
    app_handle: &AppHandle,
    paste_method: &PasteMethod,
    paste_delay_ms: u64,
) -> Result<(), String> {
    match paste_method {
        PasteMethod::None => {
            info!("PasteMethod::None selected - skipping paste action");
        }
        PasteMethod::Direct => {
            paste_direct(enigo, text)?;
        }
        PasteMethod::CtrlV | PasteMethod::CtrlShiftV | PasteMethod::ShiftInsert => {
            paste_via_clipboard(enigo, text, app_handle, paste_method, paste_delay_ms)?;
        }
    }
    Ok(())
}
