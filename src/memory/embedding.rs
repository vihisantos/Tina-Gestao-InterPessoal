//! Embedding generation for semantic search

/// Embedding generator trait.
pub trait EmbeddingGenerator: Send + Sync {
    /// Generate embeddings for text.
    fn embed(&self, text: &str) -> Vec<f32>;
    
    /// Get the embedding dimension.
    fn dimension(&self) -> usize;
}

/// Local embedding generator (for future implementation).
pub struct LocalEmbedder {
    dimension: usize,
}

impl LocalEmbedder {
    /// Create a new local embedder.
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl EmbeddingGenerator for LocalEmbedder {
    fn embed(&self, _text: &str) -> Vec<f32> {
        // TODO: Implement local embedding with candle or tch
        vec![0.0; self.dimension]
    }
    
    fn dimension(&self) -> usize {
        self.dimension
    }
}