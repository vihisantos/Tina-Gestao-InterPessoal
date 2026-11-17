//! MCP (Model Context Protocol) adapter

/// Placeholder for MCP adapter implementation.
/// MCP servers can be connected to provide tools, resources, and prompts.

/// MCP server configuration.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    /// Server name
    pub name: String,
    /// Type: stdio or http
    pub r#type: McpTransportType,
    /// Command to run server (for stdio)
    pub command: Option<String>,
    /// Arguments for server
    pub args: Vec<String>,
    /// URL for HTTP server
    pub url: Option<String>,
}

/// MCP transport type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum McpTransportType {
    /// Stdio transport
    Stdio,
    /// HTTP transport
    Http,
}

/// MCP adapter for connecting to servers.
pub struct McpAdapter {
    servers: Vec<McpServerConfig>,
}

impl McpAdapter {
    /// Create a new MCP adapter.
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }
    
    /// Add a server configuration.
    pub fn add_server(&mut self, config: McpServerConfig) {
        self.servers.push(config);
    }
    
    /// List configured servers.
    pub fn list_servers(&self) -> &[McpServerConfig] {
        &self.servers
    }
}

impl Default for McpAdapter {
    fn default() -> Self {
        Self::new()
    }
}