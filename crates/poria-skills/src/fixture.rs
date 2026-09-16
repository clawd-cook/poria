/// Returns `true` when the environment variable `PORIA_PIPELINE_FIXTURE` is set to `"1"`.
pub fn is_fixture_mode() -> bool {
    std::env::var("PORIA_PIPELINE_FIXTURE")
        .map(|v| v == "1")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_mode_off_by_default() {
        // Unless the env var is explicitly set to "1", should be false.
        // We cannot guarantee it is unset in CI, so this is a best-effort check.
        let val = std::env::var("PORIA_PIPELINE_FIXTURE").unwrap_or_default();
        if val != "1" {
            assert!(!is_fixture_mode());
        }
    }
}
