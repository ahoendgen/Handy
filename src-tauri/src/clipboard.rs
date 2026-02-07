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
#[derive(Debug, Clone, PartialEq)]
pub enum OutputSegment {
    Text(String),
    KeyPress(String),
}

/// Process text through trigger word replacement.
/// Returns segments to be output (text and key presses interleaved).
pub fn process_trigger_words(text: &str, trigger_words: &[TriggerWord]) -> Vec<OutputSegment> {
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
        let remaining_lower = remaining.to_lowercase();

        // Find the trigger with the earliest position in the text
        let mut earliest_match: Option<(usize, &TriggerWord)> = None;

        for trigger in &triggers {
            let phrase_lower = trigger.trigger_phrase.to_lowercase();

            if let Some(pos) = find_word_boundary_match(&remaining_lower, &phrase_lower) {
                match &earliest_match {
                    None => earliest_match = Some((pos, trigger)),
                    Some((earliest_pos, earliest_trigger)) => {
                        // Prefer earlier position, or longer phrase at same position
                        if pos < *earliest_pos
                            || (pos == *earliest_pos
                                && trigger.trigger_phrase.len()
                                    > earliest_trigger.trigger_phrase.len())
                        {
                            earliest_match = Some((pos, trigger));
                        }
                    }
                }
            }
        }

        match earliest_match {
            Some((pos, trigger)) => {
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
            }
            None => {
                // No trigger found - add remaining text and break
                segments.push(OutputSegment::Text(remaining));
                break;
            }
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

/// Execute a list of output segments (text and key presses).
fn execute_segments(
    segments: &[OutputSegment],
    enigo: &mut Enigo,
    app_handle: &AppHandle,
    paste_method: &PasteMethod,
    paste_delay_ms: u64,
) -> Result<(), String> {
    for segment in segments {
        match segment {
            OutputSegment::Text(t) if !t.is_empty() => {
                info!("Pasting text segment: '{}'", t);
                paste_segment(enigo, t, app_handle, paste_method, paste_delay_ms)?;
                // Wait for paste to complete before processing next segment
                std::thread::sleep(Duration::from_millis(100));
            }
            OutputSegment::KeyPress(key) => {
                info!("Sending key press: '{}'", key);
                // Try Linux native tools first, fall back to enigo
                #[cfg(target_os = "linux")]
                {
                    if !send_key_press_linux(key)? {
                        send_key_press(enigo, key)?;
                    }
                }
                #[cfg(not(target_os = "linux"))]
                send_key_press(enigo, key)?;

                // Delay after key press to let the application process it
                std::thread::sleep(Duration::from_millis(50));
            }
            _ => {}
        }
    }
    Ok(())
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
        info!(
            "Trigger words enabled, processing {} segments",
            segments.len()
        );
        execute_segments(&segments, &mut enigo, &app_handle, &paste_method, paste_delay_ms)?;
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

/// Paste pre-processed segments directly, bypassing trigger word detection.
/// Use this when trigger words were extracted earlier in the pipeline
/// (e.g., before LLM post-processing) and segments are ready to paste.
pub fn paste_with_segments(
    mut segments: Vec<OutputSegment>,
    text_for_clipboard: &str,
    app_handle: AppHandle,
) -> Result<(), String> {
    let settings = get_settings(&app_handle);
    let paste_method = settings.paste_method;
    let paste_delay_ms = settings.paste_delay_ms;

    info!(
        "Using paste method: {:?}, delay: {}ms (pre-extracted segments)",
        paste_method, paste_delay_ms
    );

    // Append trailing space to last text segment if configured
    if settings.append_trailing_space {
        for seg in segments.iter_mut().rev() {
            if let OutputSegment::Text(ref mut t) = seg {
                t.push(' ');
                break;
            }
        }
    }

    // Get the managed Enigo instance
    let enigo_state = app_handle
        .try_state::<EnigoState>()
        .ok_or("Enigo state not initialized")?;
    let mut enigo = enigo_state
        .0
        .lock()
        .map_err(|e| format!("Failed to lock Enigo: {}", e))?;

    info!(
        "Processing {} pre-extracted segments",
        segments.len()
    );
    execute_segments(&segments, &mut enigo, &app_handle, &paste_method, paste_delay_ms)?;

    // After pasting, optionally copy clean text to clipboard
    if settings.clipboard_handling == ClipboardHandling::CopyToClipboard {
        let clipboard = app_handle.clipboard();
        clipboard
            .write_text(text_for_clipboard)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trigger(phrase: &str, action_type: TriggerActionType, action_value: &str) -> TriggerWord {
        TriggerWord {
            id: format!("test_{}", phrase),
            trigger_phrase: phrase.to_string(),
            action_type,
            action_value: action_value.to_string(),
            enabled: true,
            is_builtin: false,
        }
    }

    #[test]
    fn test_find_word_boundary_match_basic() {
        assert_eq!(find_word_boundary_match("hello enter world", "enter"), Some(6));
        assert_eq!(find_word_boundary_match("enter world", "enter"), Some(0));
        assert_eq!(find_word_boundary_match("hello enter", "enter"), Some(6));
    }

    #[test]
    fn test_find_word_boundary_match_no_partial() {
        // Should NOT match partial words
        assert_eq!(find_word_boundary_match("entering the room", "enter"), None);
        assert_eq!(find_word_boundary_match("center of attention", "enter"), None);
        assert_eq!(find_word_boundary_match("reenter the building", "enter"), None);
    }

    #[test]
    fn test_find_word_boundary_match_with_punctuation() {
        assert_eq!(find_word_boundary_match("hello enter, world", "enter"), Some(6));
        assert_eq!(find_word_boundary_match("hello enter. world", "enter"), Some(6));
        assert_eq!(find_word_boundary_match("enter!", "enter"), Some(0));
    }

    #[test]
    fn test_process_trigger_words_single_keypress() {
        let triggers = vec![
            make_trigger("enter", TriggerActionType::KeyPress, "enter"),
        ];

        let segments = process_trigger_words("hello enter world", &triggers);
        assert_eq!(segments.len(), 3);

        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "hello"),
            _ => panic!("Expected text segment"),
        }
        match &segments[1] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment"),
        }
        match &segments[2] {
            OutputSegment::Text(t) => assert_eq!(t, "world"),
            _ => panic!("Expected text segment"),
        }
    }

    #[test]
    fn test_process_trigger_words_text_replacement() {
        let triggers = vec![
            make_trigger("period", TriggerActionType::TextReplacement, "."),
        ];

        let segments = process_trigger_words("hello period", &triggers);
        assert_eq!(segments.len(), 1);

        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "hello ."),
            _ => panic!("Expected merged text segment"),
        }
    }

    #[test]
    fn test_process_trigger_words_punctuation_after_keypress() {
        let triggers = vec![
            make_trigger("enter", TriggerActionType::KeyPress, "enter"),
        ];

        // Punctuation after keypress trigger should be ignored
        let segments = process_trigger_words("hello enter. world", &triggers);
        assert_eq!(segments.len(), 3);

        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "hello"),
            _ => panic!("Expected text segment"),
        }
        match &segments[1] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment"),
        }
        match &segments[2] {
            OutputSegment::Text(t) => assert_eq!(t, "world"),
            _ => panic!("Expected text segment"),
        }
    }

    #[test]
    fn test_process_trigger_words_at_end() {
        let triggers = vec![
            make_trigger("enter", TriggerActionType::KeyPress, "enter"),
        ];

        let segments = process_trigger_words("submit enter", &triggers);
        assert_eq!(segments.len(), 2);

        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "submit"),
            _ => panic!("Expected text segment"),
        }
        match &segments[1] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment"),
        }
    }

    #[test]
    fn test_process_trigger_words_disabled_trigger() {
        let mut trigger = make_trigger("enter", TriggerActionType::KeyPress, "enter");
        trigger.enabled = false;
        let triggers = vec![trigger];

        let segments = process_trigger_words("hello enter world", &triggers);
        assert_eq!(segments.len(), 1);

        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "hello enter world"),
            _ => panic!("Expected text segment"),
        }
    }

    #[test]
    fn test_process_trigger_words_case_insensitive() {
        let triggers = vec![
            make_trigger("enter", TriggerActionType::KeyPress, "enter"),
        ];

        let segments = process_trigger_words("hello ENTER world", &triggers);
        assert_eq!(segments.len(), 3);

        match &segments[1] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment"),
        }
    }

    #[test]
    fn test_process_trigger_words_same_trigger_multiple_times() {
        let triggers = vec![
            make_trigger("enter", TriggerActionType::KeyPress, "enter"),
        ];

        let segments = process_trigger_words("first paragraph enter second paragraph enter third paragraph", &triggers);
        // Should be: Text, KeyPress, Text, KeyPress, Text = 5 segments
        assert_eq!(segments.len(), 5, "Expected 5 segments, got: {:?}", segments);

        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "first paragraph"),
            _ => panic!("Expected text segment, got {:?}", segments[0]),
        }
        match &segments[1] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment, got {:?}", segments[1]),
        }
        match &segments[2] {
            OutputSegment::Text(t) => assert_eq!(t, "second paragraph"),
            _ => panic!("Expected text segment, got {:?}", segments[2]),
        }
        match &segments[3] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment, got {:?}", segments[3]),
        }
        match &segments[4] {
            OutputSegment::Text(t) => assert_eq!(t, "third paragraph"),
            _ => panic!("Expected text segment, got {:?}", segments[4]),
        }
    }

    #[test]
    fn test_process_trigger_words_same_trigger_three_times_consecutive() {
        let triggers = vec![
            make_trigger("enter", TriggerActionType::KeyPress, "enter"),
        ];

        // Three enters in a row
        let segments = process_trigger_words("hello enter enter enter world", &triggers);
        assert_eq!(segments.len(), 5, "Expected 5 segments, got: {:?}", segments);

        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "hello"),
            _ => panic!("Expected text segment"),
        }
        match &segments[1] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment"),
        }
        match &segments[2] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment"),
        }
        match &segments[3] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment"),
        }
        match &segments[4] {
            OutputSegment::Text(t) => assert_eq!(t, "world"),
            _ => panic!("Expected text segment"),
        }
    }

    #[test]
    fn test_process_trigger_words_text_replacement_multiple_times() {
        let triggers = vec![
            make_trigger("period", TriggerActionType::TextReplacement, "."),
        ];

        let segments = process_trigger_words("first sentence period second sentence period third sentence", &triggers);
        assert_eq!(segments.len(), 1, "All text replacements should merge into one text segment, got: {:?}", segments);

        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "first sentence . second sentence . third sentence"),
            _ => panic!("Expected merged text segment"),
        }
    }

    #[test]
    fn test_process_trigger_words_multiple_triggers() {
        let triggers = vec![
            make_trigger("enter", TriggerActionType::KeyPress, "enter"),
            make_trigger("tab", TriggerActionType::KeyPress, "tab"),
        ];

        let segments = process_trigger_words("name tab email enter", &triggers);
        assert_eq!(segments.len(), 4);

        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "name"),
            _ => panic!("Expected text segment"),
        }
        match &segments[1] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "tab"),
            _ => panic!("Expected keypress segment"),
        }
        match &segments[2] {
            OutputSegment::Text(t) => assert_eq!(t, "email"),
            _ => panic!("Expected text segment"),
        }
        match &segments[3] {
            OutputSegment::KeyPress(k) => assert_eq!(k, "enter"),
            _ => panic!("Expected keypress segment"),
        }
    }

    /// Simulates the full pipeline: extract triggers from raw transcription,
    /// join text for LLM processing, simulate LLM output, reconstruct segments.
    #[test]
    fn test_pipeline_extract_then_reconstruct_with_llm() {
        let triggers = vec![
            make_trigger("enter", TriggerActionType::KeyPress, "enter"),
        ];

        // Step 1: Raw transcription with trigger words
        let raw = "first paragraph enter second paragraph enter third paragraph";
        let segments = process_trigger_words(raw, &triggers);
        assert_eq!(segments.len(), 5);

        // Step 2: Extract text parts for LLM processing
        let text_parts: Vec<&str> = segments
            .iter()
            .filter_map(|s| match s {
                OutputSegment::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        let clean_text = text_parts.join("\n");
        assert_eq!(clean_text, "first paragraph\nsecond paragraph\nthird paragraph");

        // Step 3: Simulate LLM post-processing (capitalizes, adds periods)
        let llm_output = "First paragraph.\nSecond paragraph.\nThird paragraph.";

        // Step 4: Reconstruct segments with LLM-processed text
        let text_count = segments
            .iter()
            .filter(|s| matches!(s, OutputSegment::Text(_)))
            .count();
        let processed_parts: Vec<&str> = llm_output.split('\n').collect();
        assert_eq!(processed_parts.len(), text_count);

        let mut part_iter = processed_parts.into_iter();
        let reconstructed: Vec<OutputSegment> = segments
            .iter()
            .map(|s| match s {
                OutputSegment::Text(_) => {
                    OutputSegment::Text(part_iter.next().unwrap().to_string())
                }
                OutputSegment::KeyPress(k) => OutputSegment::KeyPress(k.clone()),
            })
            .collect();

        // Verify: LLM-processed text with keypresses at original positions
        assert_eq!(reconstructed.len(), 5);
        assert_eq!(reconstructed[0], OutputSegment::Text("First paragraph.".to_string()));
        assert_eq!(reconstructed[1], OutputSegment::KeyPress("enter".to_string()));
        assert_eq!(reconstructed[2], OutputSegment::Text("Second paragraph.".to_string()));
        assert_eq!(reconstructed[3], OutputSegment::KeyPress("enter".to_string()));
        assert_eq!(reconstructed[4], OutputSegment::Text("Third paragraph.".to_string()));
    }

    /// Tests that text replacements (like "period" -> ".") are applied before
    /// the text would go to LLM, so the LLM sees proper punctuation.
    #[test]
    fn test_text_replacements_applied_before_llm() {
        let triggers = vec![
            make_trigger("period", TriggerActionType::TextReplacement, "."),
            make_trigger("comma", TriggerActionType::TextReplacement, ","),
        ];

        let raw = "hello period how are you comma I am fine";
        let segments = process_trigger_words(raw, &triggers);

        // All text replacements merge into one text segment
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            OutputSegment::Text(t) => assert_eq!(t, "hello . how are you , I am fine"),
            _ => panic!("Expected text segment"),
        }
    }
}
