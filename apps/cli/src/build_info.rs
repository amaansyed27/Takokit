/// Build identifier shared by the `tako` and `takokit` binaries.
///
/// Resolution order is implemented by `apps/cli/build.rs`:
/// `TAKOKIT_BUILD_ID`, the current Git commit, then a stable version fallback.
pub const BUILD_ID: &str = env!("TAKOKIT_BUILD_ID");

pub fn build_id() -> &'static str {
    BUILD_ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_identifier_is_never_empty() {
        assert!(!build_id().trim().is_empty());
    }
}
