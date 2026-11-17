//! MCP server implementation

/// Placeholder for MCP server implementation.
/// MCP servers can expose tools, resources, and prompts.

/// MCP server configuration.
#[derive(Debug, Clone)]
pub struct McpServer {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
}

impl McpServer {
    /// Create a new MCP server.
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
        }
    }
}