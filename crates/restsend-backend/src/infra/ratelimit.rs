//! In-memory rate limiters (Go `ratelimiter.go` parity subset).
//!
//! Fixed-window-per-second counters, keyed by arbitrary strings (user id,
//! client IP). Zero rate disables limiting at the call site.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

pub struct RateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
    started_at: Instant,
}

struct Bucket {
    window: u64,
    count: u32,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            started_at: Instant::now(),
        }
    }

    /// Returns true when the request is allowed within `rate` per second.
    pub fn allow(&self, key: &str, rate: u64) -> bool {
        if rate == 0 {
            return true;
        }
        let current_window = self.started_at.elapsed().as_secs();
        let mut buckets = self.buckets.lock().unwrap();
        // opportunistic cleanup when the map grows large
        if buckets.len() > 65536 {
            buckets.retain(|_, bucket| bucket.window + 2 >= current_window);
        }
        let bucket = buckets.entry(key.to_string()).or_insert(Bucket {
            window: current_window,
            count: 0,
        });
        if bucket.window != current_window {
            bucket.window = current_window;
            bucket.count = 0;
        }
        if bucket.count >= rate as u32 {
            return false;
        }
        bucket.count += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_within_rate_and_rejects_above() {
        let limiter = RateLimiter::new();
        for _ in 0..3 {
            assert!(limiter.allow("u1", 3));
        }
        assert!(!limiter.allow("u1", 3));
        // independent keys
        assert!(limiter.allow("u2", 3));
    }

    #[test]
    fn zero_rate_disables_limiting() {
        let limiter = RateLimiter::new();
        for _ in 0..1000 {
            assert!(limiter.allow("u1", 0));
        }
    }
}
