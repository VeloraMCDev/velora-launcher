//! Legacy per-name failed-login throttle shared by account and game authentication.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const LOCKOUT_MESSAGE: &str = "too many failed attempts, try again in a few minutes";

#[derive(Default)]
pub struct LoginGuard {
    failures: Mutex<HashMap<String, (u32, Instant)>>,
}

impl LoginGuard {
    const MAX: u32 = 10;
    const WINDOW: Duration = Duration::from_secs(300);

    pub fn check(&self, username: &str) -> Result<(), &'static str> {
        self.check_at(username, Instant::now())
    }

    fn check_at(&self, username: &str, now: Instant) -> Result<(), &'static str> {
        let map = self.failures.lock().unwrap();
        if let Some((count, since)) = map.get(&username.to_lowercase()) {
            if *count >= Self::MAX && now.duration_since(*since) < Self::WINDOW {
                return Err(LOCKOUT_MESSAGE);
            }
        }
        Ok(())
    }

    pub fn fail(&self, username: &str) {
        self.fail_at(username, Instant::now());
    }

    fn fail_at(&self, username: &str, now: Instant) {
        let mut map = self.failures.lock().unwrap();
        let entry = map.entry(username.to_lowercase()).or_insert((0, now));
        if now.duration_since(entry.1) >= Self::WINDOW {
            *entry = (0, now);
        }
        entry.0 += 1;
    }

    pub fn succeed(&self, username: &str) {
        self.failures.lock().unwrap().remove(&username.to_lowercase());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockout_is_case_insensitive_per_name_and_success_resets_it() {
        let guard = LoginGuard::default();
        let now = Instant::now();
        for _ in 0..9 {
            guard.fail_at("ExamplePlayer", now);
        }
        assert_eq!(guard.check_at("EXAMPLEPLAYER", now), Ok(()));
        guard.fail_at("exampleplayer", now);
        assert_eq!(guard.check_at("ExamplePlayer", now), Err(LOCKOUT_MESSAGE));
        assert_eq!(guard.check_at("OtherPlayer", now), Ok(()));
        guard.succeed("EXAMPLEPLAYER");
        assert_eq!(guard.check_at("ExamplePlayer", now), Ok(()));
    }

    #[test]
    fn window_expires_at_five_minutes_and_next_failure_starts_a_new_window() {
        let guard = LoginGuard::default();
        let now = Instant::now();
        for _ in 0..10 {
            guard.fail_at("ExamplePlayer", now);
        }
        assert_eq!(guard.check_at("ExamplePlayer", now + Duration::from_secs(299)), Err(LOCKOUT_MESSAGE));
        let next = now + Duration::from_secs(300);
        assert_eq!(guard.check_at("ExamplePlayer", next), Ok(()));
        guard.fail_at("ExamplePlayer", next);
        for _ in 0..8 {
            guard.fail_at("ExamplePlayer", next);
        }
        assert_eq!(guard.check_at("ExamplePlayer", next), Ok(()));
        guard.fail_at("ExamplePlayer", next);
        assert_eq!(guard.check_at("ExamplePlayer", next + Duration::from_secs(299)), Err(LOCKOUT_MESSAGE));
        assert_eq!(guard.check_at("ExamplePlayer", next + Duration::from_secs(300)), Ok(()));
    }
}
