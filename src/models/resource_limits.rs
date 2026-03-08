use serde::{Deserialize, Serialize};
use std::fmt;

/// Resource limits for Docker containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Memory limit in megabytes
    pub memory_mb: u32,

    /// CPU shares (0-1024, where 1024 is 1 CPU)
    pub cpu_shares: f64,

    /// Maximum number of processes
    pub pids_limit: u32,

    /// Block I/O weight (100-1000)
    pub blkio_weight: Option<u16>,

    /// Network bandwidth limit in Mbps
    pub network_limit_mbps: Option<u32>,

    /// Disk space limit in gigabytes
    pub disk_limit_gb: Option<u32>,

    /// GPU count (0 = no GPU limit)
    pub gpu_count: u32,

    /// GPU memory limit in megabytes
    pub gpu_memory_mb: Option<u32>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: 512,  // 512MB default memory
            cpu_shares: 1.0, // 1 CPU default
            pids_limit: 100, // 100 processes default
            blkio_weight: None,
            network_limit_mbps: None,
            disk_limit_gb: None,
            gpu_count: 0,
            gpu_memory_mb: None,
        }
    }
}

impl ResourceLimits {
    /// Create new resource limits with sensible defaults
    pub fn new(memory_mb: u32, cpu_shares: f64) -> Self {
        Self {
            memory_mb,
            cpu_shares,
            ..Default::default()
        }
    }

    /// Create minimal resource limits for testing/development
    pub fn minimal() -> Self {
        Self {
            memory_mb: 128,
            cpu_shares: 0.5,
            pids_limit: 50,
            ..Default::default()
        }
    }

    /// Create standard resource limits for production
    pub fn standard() -> Self {
        Self {
            memory_mb: 1024,
            cpu_shares: 2.0,
            pids_limit: 200,
            ..Default::default()
        }
    }

    /// Create high-performance resource limits for heavy workloads
    pub fn high_performance() -> Self {
        Self {
            memory_mb: 4096,
            cpu_shares: 4.0,
            pids_limit: 500,
            ..Default::default()
        }
    }

    /// Set memory limit
    pub fn with_memory(mut self, memory_mb: u32) -> Self {
        self.memory_mb = memory_mb;
        self
    }

    /// Set CPU shares
    pub fn with_cpu(mut self, cpu_shares: f64) -> Self {
        self.cpu_shares = cpu_shares;
        self
    }

    /// Set PIDs limit
    pub fn with_pids_limit(mut self, pids_limit: u32) -> Self {
        self.pids_limit = pids_limit;
        self
    }

    /// Set GPU count
    pub fn with_gpu_count(mut self, gpu_count: u32) -> Self {
        self.gpu_count = gpu_count;
        self
    }

    /// Set GPU memory limit
    pub fn with_gpu_memory(mut self, gpu_memory_mb: u32) -> Self {
        self.gpu_memory_mb = Some(gpu_memory_mb);
        self
    }

    /// Validate resource limits
    pub fn validate(&self) -> Result<(), ResourceLimitsError> {
        if self.memory_mb == 0 {
            return Err(ResourceLimitsError::InvalidMemory(
                "Memory limit cannot be zero".to_string(),
            ));
        }

        if self.memory_mb > 65536 {
            return Err(ResourceLimitsError::InvalidMemory(
                "Memory limit too high (max 64GB)".to_string(),
            ));
        }

        if self.cpu_shares <= 0.0 {
            return Err(ResourceLimitsError::InvalidCPU(
                "CPU shares must be positive".to_string(),
            ));
        }

        if self.cpu_shares > 64.0 {
            return Err(ResourceLimitsError::InvalidCPU(
                "CPU shares too high (max 64)".to_string(),
            ));
        }

        if self.pids_limit == 0 {
            return Err(ResourceLimitsError::InvalidPids(
                "PIDs limit cannot be zero".to_string(),
            ));
        }

        if self.pids_limit > 10000 {
            return Err(ResourceLimitsError::InvalidPids(
                "PIDs limit too high (max 10000)".to_string(),
            ));
        }

        if let Some(blkio_weight) = self.blkio_weight {
            if blkio_weight < 10 || blkio_weight > 1000 {
                return Err(ResourceLimitsError::InvalidBlkio(
                    "Block I/O weight must be between 10-1000".to_string(),
                ));
            }
        }

        if let Some(network_limit) = self.network_limit_mbps {
            if network_limit > 10000 {
                return Err(ResourceLimitsError::InvalidNetwork(
                    "Network limit too high (max 10000 Mbps)".to_string(),
                ));
            }
        }

        if let Some(disk_limit) = self.disk_limit_gb {
            if disk_limit > 1000 {
                return Err(ResourceLimitsError::InvalidDisk(
                    "Disk limit too high (max 1000 GB)".to_string(),
                ));
            }
        }

        if let Some(gpu_memory) = self.gpu_memory_mb {
            if gpu_memory > 81920 {
                return Err(ResourceLimitsError::InvalidGpuMemory(
                    "GPU memory limit too high (max 80GB)".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Get total memory in bytes
    pub fn memory_bytes(&self) -> u64 {
        (self.memory_mb as u64) * 1024 * 1024
    }

    /// Get CPU limit as percentage (assuming 1 CPU = 100%)
    pub fn cpu_percentage(&self) -> u8 {
        (self.cpu_shares * 100.0) as u8
    }

    /// Check if limits are suitable for development
    pub fn is_development_profile(&self) -> bool {
        self.memory_mb <= 512 && self.cpu_shares <= 1.0
    }

    /// Check if limits are suitable for production
    pub fn is_production_profile(&self) -> bool {
        self.memory_mb >= 1024 && self.cpu_shares >= 1.0
    }

    /// Calculate a security score based on resource limits (0-100)
    pub fn security_score(&self) -> u8 {
        let mut score = 100;

        // Penalize excessive memory
        if self.memory_mb > 4096 {
            score -= 20;
        } else if self.memory_mb > 1024 {
            score -= 10;
        }

        // Penalize excessive CPU
        if self.cpu_shares > 4.0 {
            score -= 20;
        } else if self.cpu_shares > 2.0 {
            score -= 10;
        }

        // Penalize excessive PIDs
        if self.pids_limit > 500 {
            score -= 15;
        } else if self.pids_limit > 200 {
            score -= 5;
        }

        // Reward GPU limits
        if self.gpu_count > 0 {
            score += 5;
        }

        // Reward resource constraints
        if self.memory_mb <= 256 && self.cpu_shares <= 0.5 {
            score += 10;
        }

        score.max(0).min(100)
    }
}

impl fmt::Display for ResourceLimits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ResourceLimits(memory={}MB, cpu={}, pids={}, security_score={})",
            self.memory_mb,
            self.cpu_shares,
            self.pids_limit,
            self.security_score()
        )
    }
}

/// Errors related to resource limits
#[derive(Debug, thiserror::Error)]
pub enum ResourceLimitsError {
    #[error("Invalid memory limit: {0}")]
    InvalidMemory(String),

    #[error("Invalid CPU shares: {0}")]
    InvalidCPU(String),

    #[error("Invalid PIDs limit: {0}")]
    InvalidPids(String),

    #[error("Invalid block I/O weight: {0}")]
    InvalidBlkio(String),

    #[error("Invalid network limit: {0}")]
    InvalidNetwork(String),

    #[error("Invalid disk limit: {0}")]
    InvalidDisk(String),

    #[error("Invalid GPU memory limit: {0}")]
    InvalidGpuMemory(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_mb, 512);
        assert_eq!(limits.cpu_shares, 1.0);
        assert_eq!(limits.pids_limit, 100);
    }

    #[test]
    fn test_minimal_limits() {
        let limits = ResourceLimits::minimal();
        assert_eq!(limits.memory_mb, 128);
        assert_eq!(limits.cpu_shares, 0.5);
        assert_eq!(limits.pids_limit, 50);
    }

    #[test]
    fn test_standard_limits() {
        let limits = ResourceLimits::standard();
        assert_eq!(limits.memory_mb, 1024);
        assert_eq!(limits.cpu_shares, 2.0);
        assert_eq!(limits.pids_limit, 200);
    }

    #[test]
    fn test_high_performance_limits() {
        let limits = ResourceLimits::high_performance();
        assert_eq!(limits.memory_mb, 4096);
        assert_eq!(limits.cpu_shares, 4.0);
        assert_eq!(limits.pids_limit, 500);
    }

    #[test]
    fn test_builder_pattern() {
        let limits = ResourceLimits::new(256, 0.5)
            .with_pids_limit(75)
            .with_gpu_count(1)
            .with_gpu_memory(4096);

        assert_eq!(limits.memory_mb, 256);
        assert_eq!(limits.cpu_shares, 0.5);
        assert_eq!(limits.pids_limit, 75);
        assert_eq!(limits.gpu_count, 1);
        assert_eq!(limits.gpu_memory_mb, Some(4096));
    }

    #[test]
    fn test_validate_valid_limits() {
        let limits = ResourceLimits::new(512, 1.0);
        assert!(limits.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_memory() {
        let limits = ResourceLimits::new(0, 1.0);
        assert!(matches!(
            limits.validate(),
            Err(ResourceLimitsError::InvalidMemory(_))
        ));
    }

    #[test]
    fn test_validate_negative_cpu() {
        let limits = ResourceLimits::new(512, -1.0);
        assert!(matches!(
            limits.validate(),
            Err(ResourceLimitsError::InvalidCPU(_))
        ));
    }

    #[test]
    fn test_validate_zero_pids() {
        let mut limits = ResourceLimits::new(512, 1.0);
        limits.pids_limit = 0;
        assert!(matches!(
            limits.validate(),
            Err(ResourceLimitsError::InvalidPids(_))
        ));
    }

    #[test]
    fn test_memory_bytes() {
        let limits = ResourceLimits::new(1024, 1.0);
        assert_eq!(limits.memory_bytes(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_cpu_percentage() {
        let limits = ResourceLimits::new(512, 2.5);
        assert_eq!(limits.cpu_percentage(), 250);
    }

    #[test]
    fn test_security_score() {
        let minimal = ResourceLimits::minimal();
        let standard = ResourceLimits::standard();
        let high_perf = ResourceLimits::high_performance();

        assert!(minimal.security_score() > standard.security_score());
        assert!(standard.security_score() > high_perf.security_score());
    }

    #[test]
    fn test_profile_detection() {
        let minimal = ResourceLimits::minimal();
        let standard = ResourceLimits::standard();

        assert!(minimal.is_development_profile());
        assert!(!minimal.is_production_profile());
        assert!(!standard.is_development_profile());
        assert!(standard.is_production_profile());
    }

    #[test]
    fn test_display() {
        let limits = ResourceLimits::new(512, 1.0);
        let display = format!("{}", limits);
        assert!(display.contains("memory=512MB"));
        assert!(display.contains("cpu=1"));
        assert!(display.contains("pids=100"));
    }
}
