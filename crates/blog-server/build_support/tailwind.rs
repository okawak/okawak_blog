use std::time::Duration;
use topcoat::tailwind::BuildError;

pub fn retry_http<T>(
    mut render: impl FnMut() -> Result<T, BuildError>,
    mut wait: impl FnMut(Duration),
) -> Result<T, BuildError> {
    for delay in [Duration::from_secs(2), Duration::from_secs(4)] {
        match render() {
            Err(BuildError::Http(error)) => {
                println!(
                    "cargo::warning=Tailwind CLI download failed: {error}; retrying in {}s",
                    delay.as_secs()
                );
                wait(delay);
            }
            result => return result,
        }
    }
    render()
}
