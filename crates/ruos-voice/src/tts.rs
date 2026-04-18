//! Text-to-speech — speaks responses via espeak-ng.

use std::process::Command;

/// Speak text aloud using espeak-ng.
pub fn speak(text: &str) {
    if text.is_empty() { return; }

    // Trim to reasonable length for speech
    let spoken = if text.len() > 300 { &text[..300] } else { text };

    // espeak-ng with natural-sounding settings
    let _ = Command::new("espeak-ng")
        .args([
            "-v", "en",       // English voice
            "-s", "160",      // Speed (words per minute)
            "-p", "40",       // Pitch (lower = more natural)
            "-a", "80",       // Amplitude
            spoken,
        ])
        .output();
}
