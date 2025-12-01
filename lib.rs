pub mod error;
pub mod process;
pub mod gen_server;

pub use error::SwErlError;
pub use process::{Process, ProcessBuilder};
pub use gen_server::{GenServer, GenServerBehavior};

// Created using AALang and Gab.