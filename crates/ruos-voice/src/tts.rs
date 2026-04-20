//! Text-to-speech — speaks responses via espeak-ng.

use std::process::Command;

/// Speak text aloud using espeak-ng.
pub fn speak(text: &str) {
    if text.is_empty() { return; }

    // Trim to reasonable length for speech
    let spoken = if text.len() > 300 { &text[..300] } else { text };

    // Try ruos-speak (better voice), fall back to espeak-ng
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/finn".to_string());
    let ruos_speak = format!("{home}/.local/bin/ruos-speak");

    if std::path::Path::new(&ruos_speak).exists() {
        let _ = Command::new(&ruos_speak).arg(spoken).output();
    } else {
        let _ = Command::new("espeak-ng")
            .args(["-v", "en-us+f3", "-s", "145", "-p", "50", "-a", "90", spoken])
            .output();
    }
}
