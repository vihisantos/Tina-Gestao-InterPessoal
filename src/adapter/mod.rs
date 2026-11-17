//! Adapters - integrations with external systems

mod nvidia_nim;
mod mcp;

pub use nvidia_nim::*;
pub use mcp::*;