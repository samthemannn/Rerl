use crate::error::SwErlError;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

pub type ProcessId = String;
/// Messages are generic, dynamically typed, and thread-safe payloads.
pub type Message = Box<dyn Any + Send + Sync>;
/// State is generic, dynamic, and thread-safe (Arc<Mutex<...>> wraps this in the process struct).
pub type State = Box<dyn Any + Send + Sync>;

/// The internal handle for a process, holding its state and mailbox receiver.
/// This represents the central 'Actor'.
pub struct Process {
    pub pid: ProcessId,
    /// The process's internal state, modeled as AALang's 'Isolated Context'[cite: 14].
    state: Arc<Mutex<State>>,
    /// The process's asynchronous message queue, modeled as AALang's 'Shared Artifacts'[cite: 14].
    mailbox: mpsc::Receiver<Message>,
}

impl Process {
    pub fn new(pid: ProcessId, initial_state: State, mailbox: mpsc::Receiver<Message>) -> Self {
        Self {
            pid,
            state: Arc::new(Mutex::new(initial_state)),
            mailbox,
        }
    }

    /// The core process run loop.
    /// CRITICAL: Implements AALang's **Semantic Filtering** design principle[cite: 35].
    /// It relies on asynchronous `recv().await` to suspend the task until a message
    /// arrives, effectively avoiding explicit monitoring or polling of the mailbox.
    pub async fn run<F, Fut>(mut self, mut handler: F) -> Result<(), SwErlError>
    where
        F: FnMut(Arc<Mutex<State>>, Message) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), SwErlError>> + Send,
    {
        while let Some(msg) = self.mailbox.recv().await {
            // The handler performs the semantic filtering based on message content
            if let Err(e) = handler(self.state.clone(), msg).await {
                eprintln!("Process {} error: {}", self.pid, e);
                // Check if the error is recoverable before crashing/returning
                if !e.is_recoverable() {
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

/// A handle used by other entities to send messages to this process[cite: 14, 13].
#[derive(Clone)]
pub struct ProcessHandle {
    pub pid: ProcessId,
    sender: mpsc::Sender<Message>,
}

impl ProcessHandle {
    /// Sends a message asynchronously. Fails if the recipient's mailbox is closed.
    pub async fn send(&self, msg: Message) -> Result<(), SwErlError> {
        self.sender.send(msg).await.map_err(|_| SwErlError::MailboxClosed)
    }
}

/// Builder pattern for creating and spawning new processes.
pub struct ProcessBuilder {
    name: Option<String>,
}

impl ProcessBuilder {
    pub fn new() -> Self {
        Self { name: None }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Spawns the new process onto the Tokio runtime.
    pub fn spawn<F, Fut>(self, initial_state: State, handler: F) -> (ProcessId, ProcessHandle)
    where
        F: FnMut(Arc<Mutex<State>>, Message) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), SwErlError>> + Send,
    {
        let pid = self.name.unwrap_or_else(|| Uuid::new_v4().to_string());
        let (tx, rx) = mpsc::channel(100); // Mailbox channel
        
        let process = Process::new(pid.clone(), initial_state, rx);
        let handle = ProcessHandle { pid: pid.clone(), sender: tx };

        // Launch the process asynchronously
        tokio::spawn(async move {
            let _ = process.run(handler).await;
        });

        (pid, handle)
    }
}