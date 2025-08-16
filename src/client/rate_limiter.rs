use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::debug;

/// Simple rate limiter for Sci-Hub requests to respect server resources
pub struct RateLimiter {
    requests_per_second: u32,
    last_request_time: Option<Instant>,
    min_interval: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter with the specified rate (requests per second)
    pub fn new(requests_per_second: u32) -> Self {
        let min_interval = if requests_per_second > 0 {
            Duration::from_millis(1000 / u64::from(requests_per_second))
        } else {
            Duration::from_secs(1)
        };
        
        debug!("Created rate limiter: {} requests per second", requests_per_second);
        
        Self {
            requests_per_second,
            last_request_time: None,
            min_interval,
        }
    }
    
    /// Wait until it's safe to make a request (respects rate limit)
    pub async fn acquire(&mut self) {
        let now = Instant::now();
        
        if let Some(last_time) = self.last_request_time {
            let elapsed = now.duration_since(last_time);
            if elapsed < self.min_interval {
                let wait_time = self.min_interval - elapsed;
                debug!("Rate limiter: waiting {}ms", wait_time.as_millis());
                sleep(wait_time).await;
            }
        }
        
        self.last_request_time = Some(Instant::now());
        debug!("Rate limiter: request permitted");
    }
    
    /// Check if a request would be allowed without waiting
    #[must_use]
    pub fn check(&self) -> bool {
        self.last_request_time.map_or(true, |last_time| Instant::now().duration_since(last_time) >= self.min_interval)
    }
    
    /// Get the current rate limit (requests per second)
    #[must_use]
    pub const fn rate_per_second(&self) -> u32 {
        self.requests_per_second
    }
    
    /// Update the rate limit
    pub fn update_rate(&mut self, requests_per_second: u32) {
        if requests_per_second != self.requests_per_second {
            self.requests_per_second = requests_per_second;
            self.min_interval = if requests_per_second > 0 {
                Duration::from_millis(1000 / u64::from(requests_per_second))
            } else {
                Duration::from_secs(1)
            };
            
            debug!("Updated rate limiter: {} requests per second", requests_per_second);
        }
    }
    
    /// Get time until next request is allowed
    #[must_use]
    pub fn time_until_ready(&self) -> Option<Duration> {
        self.last_request_time.and_then(|last_time| {
                let elapsed = Instant::now().duration_since(last_time);
                if elapsed >= self.min_interval {
                    None
                } else {
                    Some(self.min_interval - elapsed)
                }
            })
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(1) // 1 request per second by default
    }
}

/// Configuration for rate limiting behavior
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Base requests per second
    pub requests_per_second: u32,
    /// Whether to use adaptive rate limiting based on response times
    pub adaptive: bool,
    /// Minimum rate when adapting (requests per second)
    pub min_rate: u32,
    /// Maximum rate when adapting (requests per second)
    pub max_rate: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 1,
            adaptive: false,
            min_rate: 1,
            max_rate: 5,
        }
    }
}

/// Adaptive rate limiter that adjusts based on server response times
pub struct AdaptiveRateLimiter {
    inner: RateLimiter,
    config: RateLimitConfig,
    response_times: Vec<Duration>,
    max_samples: usize,
}

impl AdaptiveRateLimiter {
    /// Create a new adaptive rate limiter
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        let inner = RateLimiter::new(config.requests_per_second);
        
        Self {
            inner,
            config,
            response_times: Vec::new(),
            max_samples: 10, // Keep last 10 response times
        }
    }
    
    /// Record a response time and potentially adjust the rate
    pub fn record_response_time(&mut self, response_time: Duration) {
        if !self.config.adaptive {
            return;
        }
        
        self.response_times.push(response_time);
        
        // Keep only the most recent samples
        if self.response_times.len() > self.max_samples {
            self.response_times.remove(0);
        }
        
        // Adjust rate based on average response time
        if self.response_times.len() >= 3 {
            let avg_response_time = self.response_times.iter().sum::<Duration>() / u32::try_from(self.response_times.len()).unwrap_or(1);
            
            let new_rate = if avg_response_time > Duration::from_millis(5000) {
                // Slow responses - decrease rate
                (self.inner.rate_per_second().saturating_sub(1)).max(self.config.min_rate)
            } else if avg_response_time < Duration::from_millis(1000) {
                // Fast responses - increase rate
                (self.inner.rate_per_second() + 1).min(self.config.max_rate)
            } else {
                // Keep current rate
                self.inner.rate_per_second()
            };
            
            if new_rate != self.inner.rate_per_second() {
                debug!("Adaptive rate limiting: adjusting from {} to {} requests/sec (avg response time: {}ms)",
                      self.inner.rate_per_second(), new_rate, avg_response_time.as_millis());
                self.inner.update_rate(new_rate);
            }
        }
    }
    
    /// Wait until it's safe to make a request
    pub async fn acquire(&mut self) {
        self.inner.acquire().await;
    }
    
    /// Check if a request would be allowed without waiting
    #[must_use]
    pub fn check(&self) -> bool {
        self.inner.check()
    }
    
    /// Get the current rate limit
    #[must_use]
    pub const fn current_rate(&self) -> u32 {
        self.inner.rate_per_second()
    }
    
    /// Get average response time from recent samples
    #[must_use]
    pub fn average_response_time(&self) -> Option<Duration> {
        if self.response_times.is_empty() {
            None
        } else {
            Some(self.response_times.iter().sum::<Duration>() / u32::try_from(self.response_times.len()).unwrap_or(1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;
    
    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let mut limiter = RateLimiter::new(2); // 2 requests per second
        
        // First request should be immediate
        let start = Instant::now();
        limiter.acquire().await;
        let first_duration = start.elapsed();
        assert!(first_duration < Duration::from_millis(100));
        
        // Second request should wait for the interval
        limiter.acquire().await;
        let second_duration = start.elapsed();
        assert!(second_duration >= Duration::from_millis(400)); // At least 500ms between requests for 2/sec
    }
    
    #[test]
    fn test_rate_limiter_check() {
        let limiter = RateLimiter::new(1);
        
        // Should be ready initially
        assert!(limiter.check());
    }
    
    #[test]
    fn test_rate_limiter_update() {
        let mut limiter = RateLimiter::new(1);
        assert_eq!(limiter.rate_per_second(), 1);
        
        limiter.update_rate(5);
        assert_eq!(limiter.rate_per_second(), 5);
    }
    
    #[test]
    fn test_adaptive_rate_limiter() {
        let config = RateLimitConfig {
            requests_per_second: 2,
            adaptive: true,
            min_rate: 1,
            max_rate: 5,
        };
        
        let mut limiter = AdaptiveRateLimiter::new(config);
        assert_eq!(limiter.current_rate(), 2);
        
        // Record slow response times
        for _ in 0..5 {
            limiter.record_response_time(Duration::from_millis(6000));
        }
        
        // Rate should decrease due to slow responses
        assert!(limiter.current_rate() < 2);
    }
    
    #[test]
    fn test_adaptive_rate_limiter_fast_responses() {
        let config = RateLimitConfig {
            requests_per_second: 2,
            adaptive: true,
            min_rate: 1,
            max_rate: 5,
        };
        
        let mut limiter = AdaptiveRateLimiter::new(config);
        
        // Record fast response times
        for _ in 0..5 {
            limiter.record_response_time(Duration::from_millis(500));
        }
        
        // Rate should increase due to fast responses
        assert!(limiter.current_rate() > 2);
    }
}