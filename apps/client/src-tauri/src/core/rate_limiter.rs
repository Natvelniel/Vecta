use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// Sliding-window RPM (Requests Per Minute) rate limiter for Gemini API calls.
/// Keeps Free Tier usage safely under quota limits without triggering HTTP 429 storms.
pub struct RpmRateLimiter {
    timestamps: Mutex<VecDeque<Instant>>,
    window: Duration,
}

impl Default for RpmRateLimiter {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}

impl RpmRateLimiter {
    pub fn new(window: Duration) -> Self {
        Self {
            timestamps: Mutex::new(VecDeque::new()),
            window,
        }
    }

    /// Returns how long the next call would need to wait under `max_rpm` (0 if immediate).
    pub async fn pending_wait(&self, max_rpm: u32) -> Duration {
        if max_rpm == 0 {
            return Duration::ZERO;
        }
        let mut guard = self.timestamps.lock().await;
        let now = Instant::now();
        while let Some(&front) = guard.front() {
            if now.duration_since(front) >= self.window {
                guard.pop_front();
            } else {
                break;
            }
        }
        if (guard.len() as u32) < max_rpm {
            return Duration::ZERO;
        }
        guard
            .front()
            .map(|&oldest| self.window.saturating_sub(now.duration_since(oldest)))
            .unwrap_or(Duration::ZERO)
    }

    /// Wait until a request slot is available under `max_rpm`, reserve the slot,
    /// and return how long this caller waited.
    pub async fn acquire(&self, max_rpm: u32) -> Duration {
        if max_rpm == 0 {
            return Duration::ZERO;
        }
        let mut total_waited = Duration::ZERO;
        loop {
            let sleep_for = {
                let mut guard = self.timestamps.lock().await;
                let now = Instant::now();
                while let Some(&front) = guard.front() {
                    if now.duration_since(front) >= self.window {
                        guard.pop_front();
                    } else {
                        break;
                    }
                }
                if (guard.len() as u32) < max_rpm {
                    guard.push_back(now);
                    return total_waited;
                }
                guard
                    .front()
                    .map(|&oldest| {
                        self.window
                            .saturating_sub(now.duration_since(oldest))
                            .saturating_add(Duration::from_millis(15))
                    })
                    .unwrap_or(Duration::from_millis(20))
            };
            tokio::time::sleep(sleep_for).await;
            total_waited = total_waited.saturating_add(sleep_for);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rpm_limiter_enforces_window_capacity() {
        let limiter = RpmRateLimiter::new(Duration::from_millis(80));
        let w1 = limiter.acquire(2).await;
        let w2 = limiter.acquire(2).await;
        assert_eq!(w1, Duration::ZERO);
        assert_eq!(w2, Duration::ZERO);

        let pending = limiter.pending_wait(2).await;
        assert!(pending > Duration::ZERO);

        let start = Instant::now();
        let w3 = limiter.acquire(2).await;
        assert!(w3 >= Duration::from_millis(40));
        assert!(start.elapsed() >= Duration::from_millis(40));
    }
}
