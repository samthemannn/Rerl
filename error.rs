use thiserror::Error;

/// Defines all possible errors in the RuErl runtime.
/// This structure aligns with the standardized error handling pattern (ex:RuErlErrorResponse).
#[derive(Error, Debug, Clone)]
pub enum SwErlError {
    /// Error indicating a target process ID or registered name was not found.
    #[error("Process not found: {0}")]
    ProcessNotFound(String),

    /// Error indicating a state downcasting failure (type mismatch).
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Error indicating the message channel receiver has closed (process has terminated).
    #[error("Mailbox closed")]
    MailboxClosed,

    /// Error indicating a synchronous 'call' timed out waiting for a response.
    #[error("Timeout waiting for response")]
    Timeout,

    /// Error indicating an asynchronous 'cast' failed to send.
    #[error("Cast failed: {0}")]
    CastFailed(String),
}

impl SwErlError {
    /// Determines if the error is recoverable, aligning with the standardized error schema.
    /// This helps processes decide whether to crash or retry/continue[cite: 119].
    pub fn is_recoverable(&self) -> bool {
        match self {
            SwErlError::ProcessNotFound(_) => true, // Recoverable: suggest retry or spawning [cite: 121]
            SwErlError::InvalidState(_) => false,   // Not recoverable: indicates logic failure
            SwErlError::MailboxClosed => false,     // Not recoverable: process is dead
            SwErlError::Timeout => true,            // Recoverable: suggest retry
            SwErlError::CastFailed(_) => false,
        }
    }
}