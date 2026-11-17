//! MCP client implementation

/// Placeholder for MCP client implementation.
/// MCP clients can connect to servers and use their tools/resources/prompts.

use crate::error::Result;

/// MCP client for connecting to servers.
pub struct McpClient {
    /// Connected servers
    servers: Vec<String>,
}

impl McpClient {
    /// Create a new MCP client.
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }
    
    /// Connect to an MCP server.
    pub async fn connect(&mut self, server: &str) -> Result<()> {
        self.servers.push(server.to_string());
        Ok(())
    }
    
    /// List connected servers.
    pub fn list_servers(&self) -> &[String] {
        &self.servers
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}