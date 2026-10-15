//! Fixed-window request limits per client address.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    per_window: u32,
    window: Duration,
    hits: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(per_window: u32, window: Duration) -> RateLimiter {
        RateLimiter { per_window, window, hits: Mutex::new(HashMap::new()) }
    }

    /// Counts one request from `ip` and says whether it's within the limit.
    pub fn allow(&self, ip: IpAddr) -> bool {
        let key = bucket(ip);
        let now = Instant::now();
        let mut hits = self.hits.lock().unwrap_or_else(|p| p.into_inner());
        if hits.len() > 50_000 {
            hits.retain(|_, (start, _)| now.duration_since(*start) < self.window);
        }
        let entry = hits.entry(key).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        if entry.1 >= self.per_window {
            return false;
        }
        entry.1 += 1;
        true
    }
}

/// One phone line usually gets a whole IPv6 /64, so that's one client.
fn bucket(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let s = v6.segments();
                IpAddr::V6([s[0], s[1], s[2], s[3], 0, 0, 0, 0].into())
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_per_address_and_v6_prefix() {
        let rl = RateLimiter::new(2, Duration::from_secs(3600));
        let a: IpAddr = "203.0.113.7".parse().unwrap();
        let b: IpAddr = "203.0.113.8".parse().unwrap();
        assert!(rl.allow(a) && rl.allow(a));
        assert!(!rl.allow(a));
        assert!(rl.allow(b));

        let v6a: IpAddr = "2001:db8:1:2::1".parse().unwrap();
        let v6b: IpAddr = "2001:db8:1:2:ffff::9".parse().unwrap();
        assert!(rl.allow(v6a) && rl.allow(v6b));
        assert!(!rl.allow(v6a), "same /64 shares one bucket");
    }

    #[test]
    fn window_resets() {
        let rl = RateLimiter::new(1, Duration::from_millis(30));
        let a: IpAddr = "198.51.100.1".parse().unwrap();
        assert!(rl.allow(a));
        assert!(!rl.allow(a));
        std::thread::sleep(Duration::from_millis(40));
        assert!(rl.allow(a));
    }
}
