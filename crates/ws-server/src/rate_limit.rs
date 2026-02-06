//! Rate limiting for WebSocket connections.
//!
//! Implements a token bucket rate limiter with per-IP and global connection limits.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limiter configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per second per IP (token bucket refill rate)
    pub requests_per_second: u32,
    /// Maximum burst size (token bucket capacity)
    pub burst_size: u32,
    /// Maximum connections per IP address
    pub max_connections_per_ip: u32,
    /// Maximum total connections across all IPs
    pub max_total_connections: u32,
    /// Window duration for cleaning up stale entries
    pub cleanup_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst_size: 200,
            max_connections_per_ip: 10,
            max_total_connections: 10000,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Token bucket state for a single IP address.
#[derive(Debug)]
struct TokenBucket {
    /// Current number of tokens available
    tokens: f64,
    /// Last time tokens were refilled
    last_refill: Instant,
    /// Active connection count for this IP
    connection_count: u32,
}

impl TokenBucket {
    fn new(initial_tokens: f64) -> Self {
        Self {
            tokens: initial_tokens,
            last_refill: Instant::now(),
            connection_count: 0,
        }
    }

    /// Refill tokens based on elapsed time and consume one if possible.
    /// Returns true if a token was consumed, false if rate limited.
    fn try_consume(&mut self, refill_rate: f64, max_tokens: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        // Refill tokens based on elapsed time
        self.tokens = (self.tokens + elapsed * refill_rate).min(max_tokens);
        self.last_refill = now;

        // Try to consume a token
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Result of a rate limit check.
#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed,
    /// Rate limited (too many requests per second)
    RateLimited,
    /// Too many connections from this IP
    TooManyConnectionsFromIp,
    /// Too many total connections globally
    TooManyTotalConnections,
}

impl RateLimitResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    pub fn error_message(&self) -> Option<&'static str> {
        match self {
            Self::Allowed => None,
            Self::RateLimited => Some("Rate limit exceeded. Please slow down."),
            Self::TooManyConnectionsFromIp => {
                Some("Too many connections from your IP address.")
            }
            Self::TooManyTotalConnections => {
                Some("Server is at maximum capacity. Please try again later.")
            }
        }
    }
}

/// Thread-safe rate limiter with per-IP tracking and global limits.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    /// Per-IP token buckets
    buckets: RwLock<HashMap<IpAddr, TokenBucket>>,
    /// Global connection counter
    total_connections: AtomicU64,
    /// Last cleanup time
    last_cleanup: RwLock<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: RwLock::new(HashMap::new()),
            total_connections: AtomicU64::new(0),
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Create a rate limiter with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Check if a new connection from the given IP is allowed.
    pub async fn check_connection(&self, ip: IpAddr) -> RateLimitResult {
        // Check global connection limit
        let total = self.total_connections.load(Ordering::Relaxed);
        if total >= self.config.max_total_connections as u64 {
            warn!(
                "Global connection limit reached: {}/{}",
                total, self.config.max_total_connections
            );
            return RateLimitResult::TooManyTotalConnections;
        }

        // Check per-IP connection limit
        let mut buckets = self.buckets.write().await;
        let bucket = buckets
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(self.config.burst_size as f64));

        if bucket.connection_count >= self.config.max_connections_per_ip {
            warn!(
                "Per-IP connection limit reached for {}: {}/{}",
                ip, bucket.connection_count, self.config.max_connections_per_ip
            );
            return RateLimitResult::TooManyConnectionsFromIp;
        }

        RateLimitResult::Allowed
    }

    /// Check if a request from the given IP is allowed (rate limiting).
    pub async fn check_request(&self, ip: IpAddr) -> RateLimitResult {
        let mut buckets = self.buckets.write().await;
        let bucket = buckets
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(self.config.burst_size as f64));

        let refill_rate = self.config.requests_per_second as f64;
        let max_tokens = self.config.burst_size as f64;

        if bucket.try_consume(refill_rate, max_tokens) {
            debug!("Request allowed for {}, tokens remaining: {:.1}", ip, bucket.tokens);
            RateLimitResult::Allowed
        } else {
            warn!("Rate limit exceeded for {}", ip);
            RateLimitResult::RateLimited
        }
    }

    /// Register a new connection from the given IP.
    /// Returns the result indicating if the connection is allowed.
    pub async fn register_connection(&self, ip: IpAddr) -> RateLimitResult {
        let result = self.check_connection(ip).await;

        if result.is_allowed() {
            let mut buckets = self.buckets.write().await;
            let bucket = buckets
                .entry(ip)
                .or_insert_with(|| TokenBucket::new(self.config.burst_size as f64));
            bucket.connection_count += 1;
            self.total_connections.fetch_add(1, Ordering::Relaxed);

            debug!(
                "Connection registered for {}: {}/{} (global: {}/{})",
                ip,
                bucket.connection_count,
                self.config.max_connections_per_ip,
                self.total_connections.load(Ordering::Relaxed),
                self.config.max_total_connections
            );
        }

        result
    }

    /// Unregister a connection from the given IP.
    pub async fn unregister_connection(&self, ip: IpAddr) {
        let mut buckets = self.buckets.write().await;
        if let Some(bucket) = buckets.get_mut(&ip) {
            bucket.connection_count = bucket.connection_count.saturating_sub(1);
            self.total_connections.fetch_sub(1, Ordering::Relaxed);

            debug!(
                "Connection unregistered for {}: {}/{} (global: {}/{})",
                ip,
                bucket.connection_count,
                self.config.max_connections_per_ip,
                self.total_connections.load(Ordering::Relaxed),
                self.config.max_total_connections
            );
        }
    }

    /// Get the current number of active connections.
    pub fn active_connections(&self) -> u64 {
        self.total_connections.load(Ordering::Relaxed)
    }

    /// Get the number of connections from a specific IP.
    pub async fn connections_from_ip(&self, ip: IpAddr) -> u32 {
        let buckets = self.buckets.read().await;
        buckets.get(&ip).map(|b| b.connection_count).unwrap_or(0)
    }

    /// Clean up stale entries (IPs with no active connections and old token state).
    pub async fn cleanup_stale_entries(&self) {
        let mut last_cleanup = self.last_cleanup.write().await;
        if last_cleanup.elapsed() < self.config.cleanup_interval {
            return;
        }
        *last_cleanup = Instant::now();
        drop(last_cleanup);

        let mut buckets = self.buckets.write().await;
        let stale_threshold = Duration::from_secs(300); // 5 minutes

        let before_count = buckets.len();
        buckets.retain(|ip, bucket| {
            let is_stale = bucket.connection_count == 0
                && bucket.last_refill.elapsed() > stale_threshold;
            if is_stale {
                debug!("Removing stale rate limit entry for {}", ip);
            }
            !is_stale
        });
        let removed = before_count - buckets.len();

        if removed > 0 {
            debug!("Cleaned up {} stale rate limit entries", removed);
        }
    }
}

/// Shared rate limiter type for use across the server.
pub type SharedRateLimiter = Arc<RateLimiter>;

/// Create a new shared rate limiter with the given configuration.
pub fn create_rate_limiter(config: RateLimitConfig) -> SharedRateLimiter {
    Arc::new(RateLimiter::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_rate_limit_allows_initial_requests() {
        let config = RateLimitConfig {
            requests_per_second: 10,
            burst_size: 5,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First 5 requests should be allowed (burst size)
        for _ in 0..5 {
            assert_eq!(limiter.check_request(ip).await, RateLimitResult::Allowed);
        }

        // 6th request should be rate limited
        assert_eq!(limiter.check_request(ip).await, RateLimitResult::RateLimited);
    }

    #[tokio::test]
    async fn test_per_ip_connection_limit() {
        let config = RateLimitConfig {
            max_connections_per_ip: 2,
            max_total_connections: 100,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // First 2 connections should be allowed
        assert_eq!(limiter.register_connection(ip).await, RateLimitResult::Allowed);
        assert_eq!(limiter.register_connection(ip).await, RateLimitResult::Allowed);

        // 3rd connection should be rejected
        assert_eq!(
            limiter.register_connection(ip).await,
            RateLimitResult::TooManyConnectionsFromIp
        );

        // After unregistering one, a new connection should be allowed
        limiter.unregister_connection(ip).await;
        assert_eq!(limiter.register_connection(ip).await, RateLimitResult::Allowed);
    }

    #[tokio::test]
    async fn test_global_connection_limit() {
        let config = RateLimitConfig {
            max_connections_per_ip: 100,
            max_total_connections: 3,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        let ip3 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));
        let ip4 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 4));

        // First 3 connections from different IPs should be allowed
        assert_eq!(limiter.register_connection(ip1).await, RateLimitResult::Allowed);
        assert_eq!(limiter.register_connection(ip2).await, RateLimitResult::Allowed);
        assert_eq!(limiter.register_connection(ip3).await, RateLimitResult::Allowed);

        // 4th connection should be rejected due to global limit
        assert_eq!(
            limiter.register_connection(ip4).await,
            RateLimitResult::TooManyTotalConnections
        );
    }

    #[tokio::test]
    async fn test_rate_limit_result_messages() {
        assert!(RateLimitResult::Allowed.error_message().is_none());
        assert!(RateLimitResult::RateLimited.error_message().is_some());
        assert!(RateLimitResult::TooManyConnectionsFromIp.error_message().is_some());
        assert!(RateLimitResult::TooManyTotalConnections.error_message().is_some());
    }
}
