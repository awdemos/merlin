use serde::{Deserialize, Serialize};
use std::fmt;

/// Network configuration for Docker containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network mode ("bridge", "host", "none", or network name)
    pub mode: String,

    /// Network aliases for the container
    pub aliases: Vec<String>,

    /// Static IP address (optional)
    pub ip_address: Option<String>,

    /// Static IPv6 address (optional)
    pub ipv6_address: Option<String>,

    /// MAC address (optional)
    pub mac_address: Option<String>,

    /// Network aliases for the container
    pub dns_servers: Vec<String>,

    /// DNS search domains
    pub dns_search_domains: Vec<String>,

    /// Extra hosts to add to /etc/hosts
    pub extra_hosts: Vec<(String, String)>,

    /// Network isolation level
    pub isolation_level: NetworkIsolationLevel,

    /// Port mappings
    pub port_mappings: Vec<PortMapping>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: "bridge".to_string(),
            aliases: Vec::new(),
            ip_address: None,
            ipv6_address: None,
            mac_address: None,
            dns_servers: Vec::new(),
            dns_search_domains: Vec::new(),
            extra_hosts: Vec::new(),
            isolation_level: NetworkIsolationLevel::Standard,
            port_mappings: Vec::new(),
        }
    }
}

impl NetworkConfig {
    /// Create a new network configuration with specified mode
    pub fn new(mode: String) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    /// Create bridge network configuration
    pub fn bridge() -> Self {
        Self::new("bridge".to_string())
    }

    /// Create host network configuration
    pub fn host() -> Self {
        Self::new("host".to_string())
    }

    /// Create isolated network configuration (no network)
    pub fn none() -> Self {
        Self::new("none".to_string())
    }

    /// Create internal network configuration
    pub fn internal(network_name: String) -> Self {
        Self {
            mode: network_name,
            isolation_level: NetworkIsolationLevel::Internal,
            ..Default::default()
        }
    }

    /// Add a network alias
    pub fn add_alias(mut self, alias: String) -> Self {
        self.aliases.push(alias);
        self
    }

    /// Set static IP address
    pub fn with_ip_address(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    /// Set static IPv6 address
    pub fn with_ipv6_address(mut self, ip: String) -> Self {
        self.ipv6_address = Some(ip);
        self
    }

    /// Set MAC address
    pub fn with_mac_address(mut self, mac: String) -> Self {
        self.mac_address = Some(mac);
        self
    }

    /// Add DNS server
    pub fn add_dns_server(mut self, dns: String) -> Self {
        self.dns_servers.push(dns);
        self
    }

    /// Add DNS search domain
    pub fn add_dns_search_domain(mut self, domain: String) -> Self {
        self.dns_search_domains.push(domain);
        self
    }

    /// Add extra host
    pub fn add_extra_host(mut self, hostname: String, ip: String) -> Self {
        self.extra_hosts.push((hostname, ip));
        self
    }

    /// Set isolation level
    pub fn with_isolation(mut self, level: NetworkIsolationLevel) -> Self {
        self.isolation_level = level;
        self
    }

    /// Add port mapping
    pub fn add_port_mapping(mut self, mapping: PortMapping) -> Self {
        self.port_mappings.push(mapping);
        self
    }

    /// Validate the network configuration
    pub fn validate(&self) -> Result<(), NetworkConfigError> {
        if self.mode.is_empty() {
            return Err(NetworkConfigError::InvalidMode(
                "Network mode cannot be empty".to_string(),
            ));
        }

        // Validate network mode
        match self.mode.as_str() {
            "bridge" | "host" | "none" => {}
            _ => {
                // Custom network name - validate format
                if !self.is_valid_network_name(&self.mode) {
                    return Err(NetworkConfigError::InvalidMode(format!(
                        "Invalid network name: {}",
                        self.mode
                    )));
                }
            }
        }

        // Validate IP addresses if provided
        if let Some(ref ip) = self.ip_address {
            if !self.is_valid_ip_address(ip) {
                return Err(NetworkConfigError::InvalidIpAddress(format!(
                    "Invalid IP address: {}",
                    ip
                )));
            }
        }

        if let Some(ref ipv6) = self.ipv6_address {
            if !self.is_valid_ipv6_address(ipv6) {
                return Err(NetworkConfigError::InvalidIpAddress(format!(
                    "Invalid IPv6 address: {}",
                    ipv6
                )));
            }
        }

        // Validate MAC address if provided
        if let Some(ref mac) = self.mac_address {
            if !self.is_valid_mac_address(mac) {
                return Err(NetworkConfigError::InvalidMacAddress(format!(
                    "Invalid MAC address: {}",
                    mac
                )));
            }
        }

        // Validate DNS servers
        for dns in &self.dns_servers {
            if !self.is_valid_ip_address(dns) {
                return Err(NetworkConfigError::InvalidDnsServer(format!(
                    "Invalid DNS server: {}",
                    dns
                )));
            }
        }

        // Validate port mappings
        for mapping in &self.port_mappings {
            mapping.validate()?;
        }

        // Validate isolation level compatibility
        if self.mode == "host" && self.isolation_level != NetworkIsolationLevel::Host {
            return Err(NetworkConfigError::IncompatibleSettings(
                "Host network mode requires host isolation level".to_string(),
            ));
        }

        if self.mode == "none" && !self.port_mappings.is_empty() {
            return Err(NetworkConfigError::IncompatibleSettings(
                "None network mode cannot have port mappings".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if network name is valid
    fn is_valid_network_name(&self, name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && name.len() <= 64
    }

    /// Check if IP address is valid
    fn is_valid_ip_address(&self, ip: &str) -> bool {
        ip.parse::<std::net::Ipv4Addr>().is_ok()
    }

    /// Check if IPv6 address is valid
    fn is_valid_ipv6_address(&self, ip: &str) -> bool {
        ip.parse::<std::net::Ipv6Addr>().is_ok()
    }

    /// Check if MAC address is valid
    fn is_valid_mac_address(&self, mac: &str) -> bool {
        mac.parse::<mac_address::MacAddress>().is_ok()
    }

    /// Generate Docker network arguments
    pub fn docker_args(&self) -> Result<Vec<String>, NetworkConfigError> {
        self.validate()?;

        let mut args = Vec::new();

        // Network mode
        args.push("--network".to_string());
        args.push(self.mode.clone());

        // Network aliases
        for alias in &self.aliases {
            args.push("--network-alias".to_string());
            args.push(alias.clone());
        }

        // IP address
        if let Some(ref ip) = self.ip_address {
            args.push("--ip".to_string());
            args.push(ip.clone());
        }

        // IPv6 address
        if let Some(ref ipv6) = self.ipv6_address {
            args.push("--ip6".to_string());
            args.push(ipv6.clone());
        }

        // MAC address
        if let Some(ref mac) = self.mac_address {
            args.push("--mac-address".to_string());
            args.push(mac.clone());
        }

        // DNS servers
        for dns in &self.dns_servers {
            args.push("--dns".to_string());
            args.push(dns.clone());
        }

        // DNS search domains
        for domain in &self.dns_search_domains {
            args.push("--dns-search".to_string());
            args.push(domain.clone());
        }

        // Extra hosts
        for (hostname, ip) in &self.extra_hosts {
            args.push("--add-host".to_string());
            args.push(format!("{}:{}", hostname, ip));
        }

        // Port mappings
        for mapping in &self.port_mappings {
            args.extend(mapping.docker_args());
        }

        Ok(args)
    }

    /// Check if this configuration provides external network access
    pub fn has_external_access(&self) -> bool {
        match self.mode.as_str() {
            "host" => true,
            "none" => false,
            "bridge" => !self.port_mappings.is_empty(),
            _ => self.isolation_level != NetworkIsolationLevel::Internal,
        }
    }

    /// Check if this configuration provides internal network access only
    pub fn is_internal_only(&self) -> bool {
        self.isolation_level == NetworkIsolationLevel::Internal
    }

    /// Calculate security score for this network configuration (0-100)
    pub fn security_score(&self) -> u8 {
        let mut score = 100;

        // Reward isolated networks
        match self.mode.as_str() {
            "none" => score += 30,
            "bridge" => score += 20,
            "host" => score -= 20,
            _ => score += 10,
        }

        // Reward isolation levels
        match self.isolation_level {
            NetworkIsolationLevel::Internal => score += 20,
            NetworkIsolationLevel::Standard => score += 10,
            NetworkIsolationLevel::Host => score -= 10,
        }

        // Penalize port mappings
        if !self.port_mappings.is_empty() {
            score -= 10;
            // Additional penalty for host port mappings
            for mapping in &self.port_mappings {
                if mapping.host_port > 1024 {
                    score -= 5;
                }
            }
        }

        // Reward DNS configuration
        if !self.dns_servers.is_empty() {
            score += 5;
        }

        // Penalty for external access
        if self.has_external_access() {
            score -= 15;
        }

        score.max(0).min(100)
    }
}

impl fmt::Display for NetworkConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NetworkConfig(mode={}, isolation={}, ports={}, security_score={})",
            self.mode,
            self.isolation_level,
            self.port_mappings.len(),
            self.security_score()
        )
    }
}

/// Network isolation levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkIsolationLevel {
    /// Host network access
    Host,
    /// Standard network isolation
    Standard,
    /// Internal network only
    Internal,
}

impl fmt::Display for NetworkIsolationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkIsolationLevel::Host => write!(f, "host"),
            NetworkIsolationLevel::Standard => write!(f, "standard"),
            NetworkIsolationLevel::Internal => write!(f, "internal"),
        }
    }
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Host port
    pub host_port: u16,

    /// Container port
    pub container_port: u16,

    /// Protocol ("tcp" or "udp")
    pub protocol: String,

    /// Host IP address to bind to (optional)
    pub host_ip: Option<String>,

    /// Publish all ports flag
    pub publish_all: bool,
}

impl Default for PortMapping {
    fn default() -> Self {
        Self {
            host_port: 0,
            container_port: 0,
            protocol: "tcp".to_string(),
            host_ip: None,
            publish_all: false,
        }
    }
}

impl PortMapping {
    /// Create a new port mapping
    pub fn new(host_port: u16, container_port: u16) -> Self {
        Self {
            host_port,
            container_port,
            ..Default::default()
        }
    }

    /// Set protocol
    pub fn with_protocol(mut self, protocol: String) -> Self {
        self.protocol = protocol;
        self
    }

    /// Set host IP
    pub fn with_host_ip(mut self, ip: String) -> Self {
        self.host_ip = Some(ip);
        self
    }

    /// Validate the port mapping
    pub fn validate(&self) -> Result<(), NetworkConfigError> {
        if self.container_port == 0 {
            return Err(NetworkConfigError::InvalidPort(
                "Container port cannot be zero".to_string(),
            ));
        }

        if self.container_port > 65535 {
            return Err(NetworkConfigError::InvalidPort(
                "Container port must be <= 65535".to_string(),
            ));
        }

        if self.host_port > 65535 {
            return Err(NetworkConfigError::InvalidPort(
                "Host port must be <= 65535".to_string(),
            ));
        }

        if self.protocol != "tcp" && self.protocol != "udp" {
            return Err(NetworkConfigError::InvalidProtocol(format!(
                "Invalid protocol: {}",
                self.protocol
            )));
        }

        if let Some(ref ip) = self.host_ip {
            if !self.is_valid_ip_address(ip) {
                return Err(NetworkConfigError::InvalidIpAddress(format!(
                    "Invalid host IP: {}",
                    ip
                )));
            }
        }

        Ok(())
    }

    /// Check if IP address is valid
    fn is_valid_ip_address(&self, ip: &str) -> bool {
        ip.parse::<std::net::Ipv4Addr>().is_ok()
    }

    /// Generate Docker port mapping arguments
    pub fn docker_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if self.publish_all {
            args.push("-P".to_string());
        } else {
            let mapping = if let Some(ref ip) = self.host_ip {
                format!(
                    "{}:{}:{}/{}",
                    ip, self.host_port, self.container_port, self.protocol
                )
            } else {
                format!(
                    "{}:{}/{}",
                    self.host_port, self.container_port, self.protocol
                )
            };
            args.push("-p".to_string());
            args.push(mapping);
        }

        args
    }
}

impl fmt::Display for PortMapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.publish_all {
            write!(f, "PortMapping(publish_all)")
        } else if let Some(ref ip) = self.host_ip {
            write!(
                f,
                "PortMapping({}:{} -> {}/{})",
                ip, self.host_port, self.container_port, self.protocol
            )
        } else {
            write!(
                f,
                "PortMapping({} -> {}/{})",
                self.host_port, self.container_port, self.protocol
            )
        }
    }
}

/// Errors related to network configuration
#[derive(Debug, thiserror::Error)]
pub enum NetworkConfigError {
    #[error("Invalid network mode: {0}")]
    InvalidMode(String),

    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(String),

    #[error("Invalid MAC address: {0}")]
    InvalidMacAddress(String),

    #[error("Invalid port: {0}")]
    InvalidPort(String),

    #[error("Invalid protocol: {0}")]
    InvalidProtocol(String),

    #[error("Invalid DNS server: {0}")]
    InvalidDnsServer(String),

    #[error("Incompatible network settings: {0}")]
    IncompatibleSettings(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_network_config() {
        let config = NetworkConfig::default();
        assert_eq!(config.mode, "bridge");
        assert!(config.aliases.is_empty());
        assert_eq!(config.isolation_level, NetworkIsolationLevel::Standard);
    }

    #[test]
    fn test_bridge_network() {
        let config = NetworkConfig::bridge();
        assert_eq!(config.mode, "bridge");
    }

    #[test]
    fn test_host_network() {
        let config = NetworkConfig::host();
        assert_eq!(config.mode, "host");
    }

    #[test]
    fn test_none_network() {
        let config = NetworkConfig::none();
        assert_eq!(config.mode, "none");
    }

    #[test]
    fn test_internal_network() {
        let config = NetworkConfig::internal("my-network".to_string());
        assert_eq!(config.mode, "my-network");
        assert_eq!(config.isolation_level, NetworkIsolationLevel::Internal);
    }

    #[test]
    fn test_with_alias() {
        let config = NetworkConfig::bridge().add_alias("web".to_string());
        assert_eq!(config.aliases, vec!["web"]);
    }

    #[test]
    fn test_with_ip_address() {
        let config = NetworkConfig::bridge().with_ip_address("192.168.1.100".to_string());
        assert_eq!(config.ip_address, Some("192.168.1.100".to_string()));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = NetworkConfig::bridge()
            .add_dns_server("8.8.8.8".to_string())
            .add_port_mapping(PortMapping::new(8080, 80));

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_mode() {
        let config = NetworkConfig::new("".to_string());
        assert!(matches!(
            config.validate(),
            Err(NetworkConfigError::InvalidMode(_))
        ));
    }

    #[test]
    fn test_validate_invalid_ip() {
        let config = NetworkConfig::bridge().with_ip_address("invalid".to_string());
        assert!(matches!(
            config.validate(),
            Err(NetworkConfigError::InvalidIpAddress(_))
        ));
    }

    #[test]
    fn test_validate_host_compatibility() {
        let config = NetworkConfig::host().with_isolation(NetworkIsolationLevel::Internal);
        assert!(matches!(
            config.validate(),
            Err(NetworkConfigError::IncompatibleSettings(_))
        ));
    }

    #[test]
    fn test_docker_args() {
        let config = NetworkConfig::bridge()
            .add_alias("web".to_string())
            .add_dns_server("8.8.8.8".to_string())
            .add_port_mapping(PortMapping::new(8080, 80));

        let args = config.docker_args().unwrap();
        assert!(args.contains(&"--network".to_string()));
        assert!(args.contains(&"bridge".to_string()));
        assert!(args.contains(&"--network-alias".to_string()));
        assert!(args.contains(&"web".to_string()));
        assert!(args.contains(&"--dns".to_string()));
        assert!(args.contains(&"8.8.8.8".to_string()));
    }

    #[test]
    fn test_external_access() {
        let bridge_config = NetworkConfig::bridge().add_port_mapping(PortMapping::new(8080, 80));
        let none_config = NetworkConfig::none();
        let host_config = NetworkConfig::host();

        assert!(bridge_config.has_external_access());
        assert!(!none_config.has_external_access());
        assert!(host_config.has_external_access());
    }

    #[test]
    fn test_security_score() {
        let secure = NetworkConfig::none();
        let insecure = NetworkConfig::host().add_port_mapping(PortMapping::new(80, 8080));

        assert!(secure.security_score() > insecure.security_score());
    }

    #[test]
    fn test_port_mapping() {
        let mapping = PortMapping::new(8080, 80)
            .with_protocol("tcp".to_string())
            .with_host_ip("127.0.0.1".to_string());

        assert_eq!(mapping.host_port, 8080);
        assert_eq!(mapping.container_port, 80);
        assert_eq!(mapping.protocol, "tcp");
        assert_eq!(mapping.host_ip, Some("127.0.0.1".to_string()));
    }

    #[test]
    fn test_port_mapping_args() {
        let mapping = PortMapping::new(8080, 80);
        let args = mapping.docker_args();
        assert_eq!(args, vec!["-p".to_string(), "8080:80/tcp".to_string()]);
    }

    #[test]
    fn test_display() {
        let config = NetworkConfig::bridge();
        let display = format!("{}", config);
        assert!(display.contains("mode=bridge"));
        assert!(display.contains("isolation=standard"));
    }
}
