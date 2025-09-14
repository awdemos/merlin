use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EmbeddingManager {
    cache: HashMap<String, Vec<f64>>,
    embedding_dim: usize,
}

impl EmbeddingManager {
    pub fn new(embedding_dim: usize) -> Self {
        EmbeddingManager {
            cache: HashMap::new(),
            embedding_dim,
        }
    }

    pub fn get_embedding(&mut self, text: &str) -> Vec<f64> {
        // Check cache first
        if let Some(embedding) = self.cache.get(text) {
            return embedding.clone();
        }

        // Generate simple embedding (placeholder for real embedding model)
        let embedding = self.generate_simple_embedding(text);

        // Cache the result
        self.cache.insert(text.to_string(), embedding.clone());

        embedding
    }

    pub fn get_embeddings_batch(&mut self, texts: &[String]) -> Vec<Vec<f64>> {
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.get_embedding(text));
        }
        embeddings
    }

    fn generate_simple_embedding(&self, text: &str) -> Vec<f64> {
        let mut embedding = vec![0.0; self.embedding_dim];

        // Simple hash-based embedding generation
        let text_bytes = text.as_bytes();
        for (i, &byte) in text_bytes.iter().enumerate() {
            let index = (i + byte as usize) % self.embedding_dim;
            embedding[index] += (byte as f64) / 255.0;
        }

        // Normalize the embedding
        let norm = embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for value in &mut embedding {
                *value /= norm;
            }
        }

        embedding
    }

    pub fn similarity(&self, embedding1: &[f64], embedding2: &[f64]) -> f64 {
        if embedding1.len() != embedding2.len() {
            return 0.0;
        }

        let mut dot_product = 0.0;
        let mut norm1 = 0.0;
        let mut norm2 = 0.0;

        for (&v1, &v2) in embedding1.iter().zip(embedding2.iter()) {
            dot_product += v1 * v2;
            norm1 += v1 * v1;
            norm2 += v2 * v2;
        }

        if norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }

        dot_product / (norm1.sqrt() * norm2.sqrt())
    }

    pub fn find_similar_prompts<'a>(&self,
                                  query_embedding: &[f64],
                                  prompt_embeddings: &'a HashMap<String, Vec<f64>>,
                                  threshold: f64) -> Vec<(&'a String, f64)> {
        let mut similarities = Vec::new();

        for (prompt, embedding) in prompt_embeddings {
            let sim = self.similarity(query_embedding, embedding);
            if sim >= threshold {
                similarities.push((prompt, sim));
            }
        }

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        similarities
    }

    pub fn cluster_prompts(&mut self, prompts: &[String], num_clusters: usize) -> Vec<Vec<String>> {
        if prompts.is_empty() || num_clusters == 0 {
            return Vec::new();
        }

        // Generate embeddings for all prompts
        let mut embeddings = Vec::new();
        for prompt in prompts {
            embeddings.push(self.get_embedding(prompt));
        }

        // Simple k-means clustering
        let mut clusters = vec![Vec::new(); num_clusters];
        let mut centroids = self.initialize_centroids(&embeddings, num_clusters);

        for _ in 0..10 { // Maximum iterations
            // Assign prompts to nearest centroid
            for (i, embedding) in embeddings.iter().enumerate() {
                let nearest_cluster = self.find_nearest_centroid(embedding, &centroids);
                clusters[nearest_cluster].push(i);
            }

            // Update centroids
            let new_centroids = self.update_centroids(&clusters, &embeddings);

            // Check for convergence
            if self.centroids_converged(&centroids, &new_centroids) {
                break;
            }

            centroids = new_centroids;

            // Clear clusters for next iteration
            for cluster in &mut clusters {
                cluster.clear();
            }
        }

        // Final assignment
        for (i, embedding) in embeddings.iter().enumerate() {
            let nearest_cluster = self.find_nearest_centroid(embedding, &centroids);
            clusters[nearest_cluster].push(i);
        }

        // Convert indices back to prompt strings
        clusters.into_iter()
            .map(|cluster| cluster.into_iter().map(|i| prompts[i].clone()).collect())
            .collect()
    }

    fn initialize_centroids(&self, embeddings: &[Vec<f64>], num_clusters: usize) -> Vec<Vec<f64>> {
        if embeddings.len() <= num_clusters {
            return embeddings.to_vec();
        }

        // Select first `num_clusters` embeddings as initial centroids
        embeddings.iter().take(num_clusters).cloned().collect()
    }

    fn find_nearest_centroid(&self, embedding: &[f64], centroids: &[Vec<f64>]) -> usize {
        let mut nearest_index = 0;
        let mut max_similarity = f64::NEG_INFINITY;

        for (i, centroid) in centroids.iter().enumerate() {
            let sim = self.similarity(embedding, centroid);
            if sim > max_similarity {
                max_similarity = sim;
                nearest_index = i;
            }
        }

        nearest_index
    }

    fn update_centroids(&self, clusters: &[Vec<usize>], embeddings: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let embedding_dim = embeddings[0].len();
        let mut new_centroids = Vec::new();

        for cluster in clusters {
            if cluster.is_empty() {
                new_centroids.push(vec![0.0; embedding_dim]);
                continue;
            }

            let mut centroid = vec![0.0; embedding_dim];
            for &idx in cluster {
                for (i, &val) in embeddings[idx].iter().enumerate() {
                    centroid[i] += val;
                }
            }

            for val in &mut centroid {
                *val /= cluster.len() as f64;
            }

            new_centroids.push(centroid);
        }

        new_centroids
    }

    fn centroids_converged(&self, old_centroids: &[Vec<f64>], new_centroids: &[Vec<f64>]) -> bool {
        if old_centroids.len() != new_centroids.len() {
            return false;
        }

        for (old, new) in old_centroids.iter().zip(new_centroids.iter()) {
            if self.similarity(old, new) < 0.99 {
                return false;
            }
        }

        true
    }

    pub fn get_cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.embedding_dim)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_creation() {
        let mut manager = EmbeddingManager::new(128);
        let text = "Hello world";
        let embedding = manager.get_embedding(text);

        assert_eq!(embedding.len(), 128);
        for value in &embedding {
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_embedding_caching() {
        let mut manager = EmbeddingManager::new(64);
        let text = "Test text";

        let embedding1 = manager.get_embedding(text);
        let embedding2 = manager.get_embedding(text);

        assert_eq!(embedding1, embedding2);
        assert_eq!(manager.get_cache_stats().0, 1);
    }

    #[test]
    fn test_similarity_calculation() {
        let mut manager = EmbeddingManager::new(32);
        let text1 = "Hello world";
        let text2 = "Hello there";
        let text3 = "Completely different text";

        let emb1 = manager.get_embedding(text1);
        let emb2 = manager.get_embedding(text2);
        let emb3 = manager.get_embedding(text3);

        let sim12 = manager.similarity(&emb1, &emb2);
        let sim13 = manager.similarity(&emb1, &emb3);

        // Similar texts should have higher similarity
        assert!(sim12 > sim13);
    }

    #[test]
    fn test_batch_embeddings() {
        let mut manager = EmbeddingManager::new(16);
        let texts = vec![
            "Text 1".to_string(),
            "Text 2".to_string(),
            "Text 3".to_string(),
        ];

        let embeddings = manager.get_embeddings_batch(&texts);

        assert_eq!(embeddings.len(), 3);
        for embedding in &embeddings {
            assert_eq!(embedding.len(), 16);
        }
    }

    #[test]
    fn test_prompt_clustering() {
        let mut manager = EmbeddingManager::new(8);
        let prompts = vec![
            "Write a function".to_string(),
            "Create a method".to_string(),
            "Design a class".to_string(),
            "Tell a story".to_string(),
            "Write a poem".to_string(),
            "Compose music".to_string(),
        ];

        let clusters = manager.cluster_prompts(&prompts, 2);

        assert_eq!(clusters.len(), 2);
        assert!(clusters.iter().all(|cluster| !cluster.is_empty()));
    }

    #[test]
    fn test_cache_management() {
        let mut manager = EmbeddingManager::new(32);

        // Add some embeddings
        manager.get_embedding("Text 1");
        manager.get_embedding("Text 2");

        let (cache_size, dim) = manager.get_cache_stats();
        assert_eq!(cache_size, 2);
        assert_eq!(dim, 32);

        manager.clear_cache();
        let (new_cache_size, _) = manager.get_cache_stats();
        assert_eq!(new_cache_size, 0);
    }
}