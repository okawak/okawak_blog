#[path = "../build_support/tailwind.rs"]
mod tailwind;

use std::{io, time::Duration};
use topcoat::tailwind::BuildError;

fn http_error() -> BuildError {
    BuildError::Http(Box::new(io::Error::from(io::ErrorKind::TimedOut).into()))
}

#[test]
fn successful_build_does_not_wait() {
    let result = tailwind::retry_http(|| Ok("stylesheet"), |_| panic!("unexpected retry"));
    assert_eq!(result.unwrap(), "stylesheet");
}

#[test]
fn http_failures_retry_with_backoff_and_return_the_successful_result() {
    let mut attempts = 0;
    let mut waits = Vec::new();
    let result = tailwind::retry_http(
        || {
            attempts += 1;
            if attempts < 3 {
                Err(http_error())
            } else {
                Ok("stylesheet")
            }
        },
        |duration| waits.push(duration),
    );
    assert_eq!(result.unwrap(), "stylesheet");
    assert_eq!(attempts, 3);
    assert_eq!(waits, [Duration::from_secs(2), Duration::from_secs(4)]);
}

#[test]
fn persistent_http_failures_stop_at_three_attempts() {
    let mut attempts = 0;
    let mut waits = Vec::new();
    let result: Result<(), _> = tailwind::retry_http(
        || {
            attempts += 1;
            Err(http_error())
        },
        |duration| waits.push(duration),
    );
    assert!(matches!(result, Err(BuildError::Http(_))));
    assert_eq!(attempts, 3);
    assert_eq!(waits, [Duration::from_secs(2), Duration::from_secs(4)]);
}

#[test]
fn non_http_errors_fail_without_retrying() {
    let mut attempts = 0;
    let result: Result<(), _> = tailwind::retry_http(
        || {
            attempts += 1;
            Err(BuildError::Io {
                path: "styles.css".into(),
                source: io::ErrorKind::NotFound.into(),
            })
        },
        |_| panic!("non-HTTP failure must not retry"),
    );
    assert!(matches!(result, Err(BuildError::Io { .. })));
    assert_eq!(attempts, 1);
}
