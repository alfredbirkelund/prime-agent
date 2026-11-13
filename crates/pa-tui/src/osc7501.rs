//! OSC 7501 Program Status Protocol: reports operational states (`idle`,
//! `working`, `blocked`, `done`, `error`) to terminals that support it (Rex,
//! Herdr). No `clear` is sent: without an `id` it removes every record on the
//! terminal, and the terminal already drops `working`/`blocked` on exit.

use std::io::{IsTerminal, Write};

/// The operational state reported via OSC 7501.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusState {
    /// Idle, awaiting user prompt or input.
    Idle,
    /// Actively running an agent turn or compaction.
    Working,
    /// Blocked awaiting confirmation or user response.
    Blocked(BlockedKind),
    /// Completed a task.
    Done,
    /// Encountered an error.
    Error,
}

/// Why the program is blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockedKind {
    Permission,
    Question,
    Auth,
}

impl StatusState {
    /// The wire string representation of the state.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Blocked(_) => "blocked",
            Self::Done => "done",
            Self::Error => "error",
        }
    }
}

impl BlockedKind {
    /// The wire string representation of the blocked kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Permission => "permission",
            Self::Question => "question",
            Self::Auth => "auth",
        }
    }
}

/// The reporting application identity for Prime Agent.
pub const APP_NAME: &str = "prime-agent";

/// Generate the OSC 7501 escape sequence for the given state.
#[must_use]
pub fn sequence(state: StatusState) -> String {
    match state {
        StatusState::Blocked(kind) => {
            let kind_str = kind.as_str();
            format!("\x1b]7501;state=blocked:kind={kind_str}:app={APP_NAME}\x1b\\")
        }
        StatusState::Idle | StatusState::Working | StatusState::Done | StatusState::Error => {
            let state_str = state.as_str();
            format!("\x1b]7501;state={state_str}:app={APP_NAME}\x1b\\")
        }
    }
}

/// Whether OSC 7501 reporting is enabled via environment variables.
/// Disabled if `TERM` is `dumb` or `linux`, or if `PRIME_AGENT_OSC7501` is
/// explicitly set to `0`, `off`, `false`, or `no`.
#[must_use]
pub fn is_enabled() -> bool {
    if std::env::var("TERM").is_ok_and(|t| t == "dumb" || t == "linux") {
        return false;
    }
    !std::env::var("PRIME_AGENT_OSC7501").is_ok_and(|val| {
        let trimmed = val.trim();
        trimmed == "0"
            || trimmed.eq_ignore_ascii_case("off")
            || trimmed.eq_ignore_ascii_case("false")
            || trimmed.eq_ignore_ascii_case("no")
    })
}

/// Emit an OSC 7501 state change to a writer.
///
/// # Errors
///
/// Returns `Err` if writing to the underlying stream fails.
pub fn emit_to<W: Write>(writer: &mut W, state: StatusState) -> std::io::Result<()> {
    let seq = sequence(state);
    writer.write_all(seq.as_bytes())?;
    writer.flush()
}

/// Emit an OSC 7501 state change directly to stdout if stdout is a terminal and reporting is enabled.
pub fn emit(state: StatusState) {
    if is_enabled() {
        let mut out = std::io::stdout();
        if out.is_terminal() {
            let _ = emit_to(&mut out, state);
        }
    }
}

/// Emit an OSC 7501 state change directly to stderr if stderr is a terminal and reporting is enabled.
pub fn emit_stderr(state: StatusState) {
    if is_enabled() {
        let mut err = std::io::stderr();
        if err.is_terminal() {
            let _ = emit_to(&mut err, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_idle_sequence() {
        assert_eq!(
            sequence(StatusState::Idle),
            "\x1b]7501;state=idle:app=prime-agent\x1b\\"
        );
    }

    #[test]
    fn format_working_sequence() {
        assert_eq!(
            sequence(StatusState::Working),
            "\x1b]7501;state=working:app=prime-agent\x1b\\"
        );
    }

    #[test]
    fn format_blocked_sequences() {
        assert_eq!(
            sequence(StatusState::Blocked(BlockedKind::Permission)),
            "\x1b]7501;state=blocked:kind=permission:app=prime-agent\x1b\\"
        );
        assert_eq!(
            sequence(StatusState::Blocked(BlockedKind::Question)),
            "\x1b]7501;state=blocked:kind=question:app=prime-agent\x1b\\"
        );
        assert_eq!(
            sequence(StatusState::Blocked(BlockedKind::Auth)),
            "\x1b]7501;state=blocked:kind=auth:app=prime-agent\x1b\\"
        );
    }

    #[test]
    fn format_done_sequence() {
        assert_eq!(
            sequence(StatusState::Done),
            "\x1b]7501;state=done:app=prime-agent\x1b\\"
        );
    }

    #[test]
    fn format_error_sequence() {
        assert_eq!(
            sequence(StatusState::Error),
            "\x1b]7501;state=error:app=prime-agent\x1b\\"
        );
    }

    #[test]
    fn emit_writes_to_buffer() {
        let mut buffer = Vec::new();
        emit_to(&mut buffer, StatusState::Working).unwrap();
        assert_eq!(buffer, b"\x1b]7501;state=working:app=prime-agent\x1b\\");
    }
}
