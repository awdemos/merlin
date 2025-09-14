// src/ab_testing/storage.rs
use crate::ab_testing::{config::ExperimentConfig, experiment::ExperimentResults};
use async_trait::async_trait;
use anyhow::Result;
use std::collections::HashMap;

#[async_trait]
pub trait ExperimentStorage: Send + Sync {
    async fn save_experiment_config(&self, config: &ExperimentConfig) -> Result<()>;
    async fn load_experiment_configs(&self) -> Result<Vec<ExperimentConfig>>;
    async fn update_experiment_config(&self, config: &ExperimentConfig) -> Result<()>;
    async fn delete_experiment_config(&self, experiment_id: &str) -> Result<bool>;

    async fn save_experiment_results(&self, results: &ExperimentResults) -> Result<()>;
    async fn load_experiment_results(&self, experiment_id: &str) -> Result<Option<ExperimentResults>>;
    async fn get_all_experiment_results(&self) -> Result<Vec<ExperimentResults>>;
}

pub struct RedisExperimentStorage {
    client: redis::Client,
}

impl RedisExperimentStorage {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    fn experiment_config_key(&self, experiment_id: &str) -> String {
        format!("ab_experiment:config:{}", experiment_id)
    }

    fn experiment_results_key(&self, experiment_id: &str) -> String {
        format!("ab_experiment:results:{}", experiment_id)
    }

    fn all_experiments_key(&self) -> &'static str {
        "ab_experiment:all"
    }
}

#[async_trait]
impl ExperimentStorage for RedisExperimentStorage {
    async fn save_experiment_config(&self, config: &ExperimentConfig) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = self.experiment_config_key(&config.id);
        let config_json = serde_json::to_string(config)?;

        redis::cmd("SET").arg(&key).arg(config_json).query_async::<_, ()>(&mut conn).await?;
        redis::cmd("SADD").arg(self.all_experiments_key()).arg(&config.id).query_async::<_, ()>(&mut conn).await?;

        Ok(())
    }

    async fn load_experiment_configs(&self) -> Result<Vec<ExperimentConfig>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let experiment_ids: Vec<String> = redis::cmd("SMEMBERS").arg(self.all_experiments_key()).query_async(&mut conn).await?;

        let mut configs = Vec::new();
        for experiment_id in experiment_ids {
            let key = self.experiment_config_key(&experiment_id);
            let config_json: Option<String> = redis::cmd("GET").arg(&key).query_async(&mut conn).await?;
            if let Some(json) = config_json {
                configs.push(serde_json::from_str(&json)?);
            }
        }

        Ok(configs)
    }

    async fn update_experiment_config(&self, config: &ExperimentConfig) -> Result<()> {
        self.save_experiment_config(config).await
    }

    async fn delete_experiment_config(&self, experiment_id: &str) -> Result<bool> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        let config_key = self.experiment_config_key(experiment_id);
        let results_key = self.experiment_results_key(experiment_id);

        redis::cmd("DEL").arg(&config_key).query_async::<_, ()>(&mut conn).await?;
        redis::cmd("DEL").arg(&results_key).query_async::<_, ()>(&mut conn).await?;
        redis::cmd("SREM").arg(self.all_experiments_key()).arg(experiment_id).query_async::<_, ()>(&mut conn).await?;

        Ok(true)
    }

    async fn save_experiment_results(&self, results: &ExperimentResults) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = self.experiment_results_key(&results.experiment_id);
        let results_json = serde_json::to_string(results)?;

        redis::cmd("SET").arg(&key).arg(results_json).query_async::<_, ()>(&mut conn).await?;
        Ok(())
    }

    async fn load_experiment_results(&self, experiment_id: &str) -> Result<Option<ExperimentResults>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = self.experiment_results_key(experiment_id);

        let results_json: Option<String> = redis::cmd("GET").arg(&key).query_async(&mut conn).await?;
        match results_json {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn get_all_experiment_results(&self) -> Result<Vec<ExperimentResults>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let experiment_ids: Vec<String> = redis::cmd("SMEMBERS").arg(self.all_experiments_key()).query_async(&mut conn).await?;

        let mut results = Vec::new();
        for experiment_id in experiment_ids {
            if let Some(result) = self.load_experiment_results(&experiment_id).await? {
                results.push(result);
            }
        }

        Ok(results)
    }
}

// In-memory storage for testing and fallback
pub struct InMemoryExperimentStorage {
    configs: std::sync::RwLock<HashMap<String, ExperimentConfig>>,
    results: std::sync::RwLock<HashMap<String, ExperimentResults>>,
}

impl InMemoryExperimentStorage {
    pub fn new() -> Self {
        Self {
            configs: std::sync::RwLock::new(HashMap::new()),
            results: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ExperimentStorage for InMemoryExperimentStorage {
    async fn save_experiment_config(&self, config: &ExperimentConfig) -> Result<()> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(config.id.clone(), config.clone());
        Ok(())
    }

    async fn load_experiment_configs(&self) -> Result<Vec<ExperimentConfig>> {
        let configs = self.configs.read().unwrap();
        Ok(configs.values().cloned().collect())
    }

    async fn update_experiment_config(&self, config: &ExperimentConfig) -> Result<()> {
        self.save_experiment_config(config).await
    }

    async fn delete_experiment_config(&self, experiment_id: &str) -> Result<bool> {
        let mut configs = self.configs.write().unwrap();
        let mut results = self.results.write().unwrap();

        Ok(configs.remove(experiment_id).is_some() || results.remove(experiment_id).is_some())
    }

    async fn save_experiment_results(&self, results: &ExperimentResults) -> Result<()> {
        let mut results_map = self.results.write().unwrap();
        results_map.insert(results.experiment_id.clone(), results.clone());
        Ok(())
    }

    async fn load_experiment_results(&self, experiment_id: &str) -> Result<Option<ExperimentResults>> {
        let results = self.results.read().unwrap();
        Ok(results.get(experiment_id).cloned())
    }

    async fn get_all_experiment_results(&self) -> Result<Vec<ExperimentResults>> {
        let results = self.results.read().unwrap();
        Ok(results.values().cloned().collect())
    }
}

impl Default for InMemoryExperimentStorage {
    fn default() -> Self {
        Self::new()
    }
}