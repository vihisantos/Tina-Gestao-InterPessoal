//! Vector store implementation for semantic search

/// Vector store for embeddings and similarity search.
pub struct VectorStore {
    /// Dimension of embeddings
    dimension: usize,
}

impl VectorStore {
    /// Create a new vector store.
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
    
    /// Calculate cosine similarity between two vectors.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
    
    /// Find top-k similar vectors (brute force for now).
    pub fn search(&self, query: &[f32], candidates: &[(String, Vec<f32>)], k: usize) -> Vec<(String, f32)> {
        let mut results: Vec<(String, f32)> = candidates
            .iter()
            .map(|(id, vec)| (id.clone(), Self::cosine_similarity(query, vec)))
            .collect();
        
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(k);
        results
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::new(768) // Default BERT embedding dimension
    }
}