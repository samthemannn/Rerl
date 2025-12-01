use crate::error::SwErlError;
use crate::process::{Message, State, ProcessBuilder, ProcessHandle};
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use std::any::Any;

/// The GenServer Behavior Protocol: defines the mandatory callbacks for an OTP server[cite: 322, 332].
#[async_trait::async_trait]
pub trait GenServerBehavior: Send + Sync + 'static + Clone {
    /// Initializes the GenServer state.
    async fn init(&self, args: Option<Message>) -> Result<State, SwErlError>;
    
    /// Handles asynchronous messages (cast) that do not require a reply.
    async fn handle_cast(&self, msg: Message, state: Arc<Mutex<State>>) -> Result<(), SwErlError>;
    
    /// Handles synchronous messages (call) and must return a reply[cite: 322, 332].
    async fn handle_call(&self, msg: Message, state: Arc<Mutex<State>>) -> Result<Message, SwErlError>;
}

// Internal wrapper structures for message differentiation
struct CastMessage(Message);
/// Wraps a call message and includes a oneshot channel sender for synchronous replies.
struct CallMessage(Message, oneshot::Sender<Result<Message, SwErlError>>);

pub struct GenServer;

impl GenServer {
    /// Starts a new GenServer process with the given behavior.
    pub async fn start<B>(behavior: B, args: Option<Message>) -> Result<ProcessHandle, SwErlError>
    where
        B: GenServerBehavior + Clone,
    {
        let initial_state = behavior.init(args).await?;
        
        let (_, handle) = ProcessBuilder::new().spawn(initial_state, move |state, msg| {
            let behavior = behavior.clone();
            async move {
                // Runtime logic uses downcasting to semantically route the message (Cast vs Call)
                if let Ok(CastMessage(inner_msg)) = msg.downcast::<CastMessage>() {
                    behavior.handle_cast(inner_msg, state).await
                } else if let Ok(CallMessage(inner_msg, reply_tx)) = msg.downcast::<CallMessage>() {
                    let result = behavior.handle_call(inner_msg, state).await;
                    // Send the result back to the caller immediately
                    let _ = reply_tx.send(result);
                    Ok(())
                } else {
                    // Ignore unknown message types
                    Ok(())
                }
            }
        });
        
        Ok(handle)
    }

    /// Sends an asynchronous message (cast) to the GenServer.
    pub async fn cast(handle: &ProcessHandle, msg: Message) -> Result<(), SwErlError> {
        handle.send(Box::new(CastMessage(msg))).await
    }

    /// Sends a synchronous message (call) and waits for a reply[cite: 322, 332].
    pub async fn call(handle: &ProcessHandle, msg: Message) -> Result<Message, SwErlError> {
        let (tx, rx) = oneshot::channel();
        handle.send(Box::new(CallMessage(msg, tx))).await?;
        
        // Block asynchronously, waiting for the server to process and reply
        match rx.await {
            Ok(res) => res,
            Err(_) => Err(SwErlError::MailboxClosed), // Oneshot closed before reply
        }
    }
}