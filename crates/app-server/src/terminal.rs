//! Terminal input handling for the TUI.
//!
//! Wraps stdin in raw mode, enables bracketed paste, and feeds byte-level
//! input through a `BracketedPasteBuffer` to produce `UiEvent` streams.

use std::io::{self, Read};
use std::os::fd::AsRawFd;

use deepseek_tui_core::{BracketedPasteBuffer, TerminalCaps, UiEvent, ansi};

/// Manages terminal input: raw mode, bracketed paste, event production.
pub struct TerminalInput {
    /// Buffered paste state.
    paste_buffer: BracketedPasteBuffer,
    /// Whether raw mode is active.
    raw_mode: bool,
    /// Saved terminal settings for restoration.
    saved_termios: Option<libc::termios>,
    /// Read buffer for partial byte sequences.
    read_buf: Vec<u8>,
    /// Detected terminal capabilities.
    caps: TerminalCaps,
}

impl TerminalInput {
    /// Create a new terminal input handler.
    /// Does NOT enable raw mode — call `enable_raw_mode()` to activate.
    #[must_use]
    pub fn new() -> Self {
        let caps = TerminalCaps::detect_from_env();
        let mut paste_buffer = BracketedPasteBuffer::new();
        paste_buffer.bracketed_supported = caps.bracketed_paste;
        // Lower burst threshold for terminals without bracketed paste
        if !caps.bracketed_paste {
            paste_buffer.burst_threshold = 20;
        }
        Self {
            paste_buffer,
            raw_mode: false,
            saved_termios: None,
            read_buf: vec![0u8; 4096],
            caps,
        }
    }

    /// Enable raw mode on stdin and activate bracketed paste.
    ///
    /// Saves the current terminal settings for restoration in `disable_raw_mode()`.
    pub fn enable_raw_mode(&mut self) -> io::Result<()> {
        if self.raw_mode {
            return Ok(());
        }

        let fd = io::stdin().as_raw_fd();
        let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };

        if unsafe { libc::tcgetattr(fd, &mut termios) } != 0 {
            return Err(io::Error::last_os_error());
        }

        self.saved_termios = Some(termios);

        // Enter raw mode: disable canonical mode, echo, signals
        let mut raw = termios;
        unsafe { libc::cfmakeraw(&mut raw) };
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }

        // Enable bracketed paste mode (skip for terminals that don't support it,
        // e.g. PuTTY, screen, tmux without explicit config)
        if self.caps.bracketed_paste {
            let enable_seq = ansi::BRACKETED_PASTE_ON;
            let mut stdout = io::stdout();
            use std::io::Write;
            stdout.write_all(enable_seq.as_bytes())?;
            stdout.flush()?;
        }

        self.raw_mode = true;
        Ok(())
    }

    /// Restore original terminal settings and disable bracketed paste.
    pub fn disable_raw_mode(&mut self) -> io::Result<()> {
        if !self.raw_mode {
            return Ok(());
        }

        // Disable bracketed paste mode (only if terminal supports it)
        if self.caps.bracketed_paste {
            let disable_seq = ansi::BRACKETED_PASTE_OFF;
            let mut stdout = io::stdout();
            use std::io::Write;
            stdout.write_all(disable_seq.as_bytes())?;
            stdout.flush()?;
        }

        // Restore saved termios
        if let Some(saved) = self.saved_termios {
            let fd = io::stdin().as_raw_fd();
            if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &saved) } != 0 {
                // Best effort — terminal might still be usable
                eprintln!("warning: failed to restore terminal settings");
            }
        }

        self.raw_mode = false;
        self.saved_termios = None;
        self.paste_buffer.reset();
        Ok(())
    }

    /// Returns true if raw mode is active.
    #[must_use]
    pub fn is_raw(&self) -> bool {
        self.raw_mode
    }

    /// Returns true if currently inside a bracketed paste region.
    #[must_use]
    pub fn is_pasting(&self) -> bool {
        self.paste_buffer.is_active()
    }

    /// Read the next batch of UI events from stdin.
    ///
    /// Blocks until at least one byte is available. Returns all events
    /// decoded from the read bytes (may include multiple KeyPressed events
    /// and zero or one PasteContent event).
    ///
    /// Returns `None` on EOF (stdin closed) or if raw mode is not active.
    pub fn read_events(&mut self) -> io::Result<Vec<UiEvent>> {
        if !self.raw_mode {
            return Ok(Vec::new());
        }

        let mut stdin = io::stdin().lock();
        let n = match stdin.read(&mut self.read_buf) {
            Ok(0) => return Ok(Vec::new()), // EOF
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        Ok(self.paste_buffer.feed_bytes(&self.read_buf[..n]))
    }

    /// Read a single event from stdin (blocking).
    ///
    /// Convenience wrapper that loops `read_events` until it produces
    /// at least one event or EOF.
    pub fn next_event(&mut self) -> io::Result<Option<UiEvent>> {
        loop {
            let events = self.read_events()?;
            if let Some(event) = events.into_iter().next() {
                return Ok(Some(event));
            }
            // read_events returned empty — might be EOF or non-event bytes
            if !self.raw_mode {
                return Ok(None);
            }
        }
    }

    /// Reset the paste buffer (useful after an interrupt).
    pub fn reset_paste(&mut self) {
        self.paste_buffer.reset();
    }
}

impl Default for TerminalInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TerminalInput {
    fn drop(&mut self) {
        let _ = self.disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_terminal_input_is_not_raw() {
        let ti = TerminalInput::new();
        assert!(!ti.is_raw());
        assert!(!ti.is_pasting());
    }

    #[test]
    fn read_events_returns_empty_when_not_raw() {
        let mut ti = TerminalInput::new();
        let events = ti.read_events().expect("read");
        assert!(events.is_empty());
    }

    #[test]
    fn paste_buffer_is_accessible() {
        let mut ti = TerminalInput::new();
        ti.reset_paste();
        assert!(!ti.is_pasting());
    }
}
