// SPDX-License-Identifier: MIT OR Apache-2.0
// Unit tests of the retry module, included from retry.rs.

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    const POLICY: RetryPolicy = RetryPolicy::new(10, 80, 5_000, 100);

    #[test]
    fn ceiling_doubles_then_caps() {
        let ms = |n| POLICY.ceiling(n).as_millis();
        assert_eq!([ms(0), ms(1), ms(2), ms(3)], [10, 20, 40, 80]);
        assert_eq!(ms(4), 80, "capped");
        assert_eq!(
            ms(200),
            80,
            "a shift past the width must saturate, not panic"
        );
    }

    #[test]
    fn jitter_stays_inside_the_ceiling_and_spreads() {
        let mut rng = fastrand::Rng::with_seed(7);
        let ceiling = POLICY.ceiling(2);
        let draws: Vec<Duration> = (0..2_000).map(|_| POLICY.jittered(2, &mut rng)).collect();
        assert!(
            draws.iter().all(|d| *d <= ceiling),
            "a draw left [0, ceiling]"
        );
        assert!(
            draws.iter().any(|d| *d < ceiling / 4),
            "no draw in the low quarter"
        );
        assert!(
            draws.iter().any(|d| *d > ceiling * 3 / 4),
            "no draw in the high quarter"
        );
    }

    #[test]
    fn jitter_differs_between_loops() {
        let mut ra = Backoff::new(POLICY).rng;
        let mut rb = Backoff::new(POLICY).rng;
        let da: Vec<u64> = (0..8).map(|_| ra.u64(..)).collect();
        let db: Vec<u64> = (0..8).map(|_| rb.u64(..)).collect();
        assert_ne!(da, db, "two loops must not share one jitter sequence");
    }

    #[test]
    fn budget_is_never_overrun() {
        let policy = RetryPolicy::new(10, 40, 120, 1_000);
        let mut backoff = Backoff::with_parts(policy, fastrand::Rng::with_seed(1), None);
        let stop = loop {
            if let Err(stop) = backoff.wait_with(|| false) {
                break stop;
            }
        };
        let elapsed = backoff.elapsed();
        assert_eq!(stop, RetryStop::DeadlineElapsed);
        assert!(
            elapsed >= Duration::from_millis(120),
            "stopped early: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(120 + 100),
            "overran the budget: {elapsed:?}"
        );
    }

    #[test]
    fn attempts_exhaust_before_the_budget() {
        let policy = RetryPolicy::new(1, 2, 60_000, 3);
        let mut backoff = Backoff::with_parts(policy, fastrand::Rng::with_seed(2), None);
        for _ in 0..3 {
            backoff
                .wait_with(|| false)
                .expect("wait within the attempt limit");
        }
        assert_eq!(
            backoff.wait_with(|| false),
            Err(RetryStop::AttemptsExhausted)
        );
        assert_eq!(backoff.attempts(), 3);
    }

    #[test]
    fn the_global_deadline_shortens_the_budget() {
        let policy = RetryPolicy::new(10, 40, 60_000, 1_000);
        let global = Some(Duration::from_millis(100));
        let mut backoff = Backoff::with_parts(policy, fastrand::Rng::with_seed(3), global);
        while backoff.wait_with(|| false).is_ok() {}
        let elapsed = backoff.elapsed();
        assert!(
            elapsed >= Duration::from_millis(100),
            "stopped early: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(100 + RETRY_GLOBAL_DEADLINE_GRACE_MS + 100),
            "ignored the global deadline: {elapsed:?}"
        );
    }

    #[test]
    fn global_deadline_with_the_cancel_flag_reports_cancelled() {
        let policy = RetryPolicy::new(10, 40, 60_000, 1_000);
        let flag = AtomicBool::new(false);
        let mut backoff = Backoff::with_parts(
            policy,
            fastrand::Rng::with_seed(4),
            Some(Duration::from_millis(60)),
        );
        let stop = loop {
            // The watchdog raises the flag when the deadline is reached.
            if backoff.elapsed() >= Duration::from_millis(60) {
                flag.store(true, Ordering::Release);
            }
            if let Err(stop) = backoff.wait_with(|| flag.load(Ordering::Acquire)) {
                break stop;
            }
        };
        assert_eq!(stop, RetryStop::Cancelled);
    }

    #[test]
    fn cancel_flag_cuts_a_long_wait_short() {
        let policy = RetryPolicy::new(10_000, 10_000, 60_000, 10);
        let mut backoff = Backoff::with_parts(policy, fastrand::Rng::with_seed(5), None);
        let flag = std::sync::Arc::new(AtomicBool::new(false));
        let raiser = {
            let flag = std::sync::Arc::clone(&flag);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(50));
                flag.store(true, Ordering::Release);
            })
        };
        let begun = Instant::now();
        let mut stop = Ok(());
        while stop.is_ok() {
            stop = backoff.wait_with(|| flag.load(Ordering::Acquire));
        }
        raiser.join().expect("raiser thread");
        assert_eq!(stop, Err(RetryStop::Cancelled));
        assert!(
            begun.elapsed() < Duration::from_millis(50 + 200),
            "cancel waited out the backoff: {:?}",
            begun.elapsed()
        );
    }

    #[test]
    fn an_armed_flag_returns_before_any_sleep() {
        let mut backoff = Backoff::with_parts(POLICY, fastrand::Rng::with_seed(6), None);
        let begun = Instant::now();
        assert_eq!(backoff.wait_with(|| true), Err(RetryStop::Cancelled));
        assert!(begun.elapsed() < Duration::from_millis(RETRY_SLICE_MS));
    }

    #[test]
    fn retry_returns_the_first_success() {
        let mut calls = 0;
        let out = retry(RetryPolicy::new(1, 2, 1_000, 10), || {
            calls += 1;
            if calls < 3 {
                Step::Retry("busy")
            } else {
                Step::Done(calls)
            }
        });
        assert_eq!(out.expect("third attempt succeeds"), 3);
    }

    #[test]
    fn retry_does_not_repeat_a_permanent_failure() {
        let mut calls = 0;
        let out: Result<(), _> = retry(RetryPolicy::new(1, 2, 1_000, 10), || {
            calls += 1;
            Step::Fail("denied")
        });
        assert!(matches!(out, Err(RetryFailure::Permanent("denied"))));
        assert_eq!(calls, 1);
    }

    #[test]
    fn retry_reports_the_last_error_when_attempts_run_out() {
        let out: Result<(), _> = retry(RetryPolicy::new(1, 2, 1_000, 2), || Step::Retry("busy"));
        match out {
            Err(RetryFailure::Stopped {
                stop,
                last,
                attempts,
            }) => {
                assert_eq!(stop, RetryStop::AttemptsExhausted);
                assert_eq!((last, attempts), ("busy", 2));
            }
            other => panic!("expected Stopped, got {other:?}"),
        }
    }
}
