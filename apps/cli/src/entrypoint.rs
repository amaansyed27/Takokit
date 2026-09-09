use std::{
    ffi::OsStr,
    time::{Duration, Instant},
};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;

pub async fn run() -> anyhow::Result<()> {
    if build_id_requested() {
        println!("{}", env!("TAKOKIT_BUILD_ID"));
        return Ok(());
    }
    if runtime_endpoint_requested() {
        print_runtime_endpoint()?;
        return Ok(());
    }

    let started = Instant::now();
    let show_timing = timing_enabled();
    let result = takokit_cli::run().await;

    if show_timing {
        let elapsed = format_duration(started.elapsed());
        match &result {
            Ok(()) => eprintln!("\nCompleted in {elapsed}"),
            Err(_) => eprintln!("\nFailed after {elapsed}"),
        }
    }

    result
}

fn build_id_requested() -> bool {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    arguments.len() == 1 && arguments[0] == OsStr::new("--build-id")
}

fn runtime_endpoint_requested() -> bool {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    arguments.len() == 1 && arguments[0] == OsStr::new("--runtime-endpoint")
}

fn print_runtime_endpoint() -> anyhow::Result<()> {
    let store = LocalStore::new(LocalStore::default_root());
    store.ensure_layout()?;
    let config = RuntimeConfig::local(store.root().to_path_buf());
    println!("{}", serde_json::to_string(&config)?);
    Ok(())
}

fn timing_enabled() -> bool {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments
        .iter()
        .any(|argument| argument == OsStr::new("--daemon-child"))
    {
        return false;
    }
    if std::env::var("TAKOKIT_OUTPUT")
        .map(|value| value.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
    {
        return false;
    }
    !arguments.windows(2).any(|pair| {
        pair[0] == OsStr::new("--output") && pair[1].to_string_lossy().eq_ignore_ascii_case("json")
    }) && !arguments.iter().any(|argument| {
        argument
            .to_string_lossy()
            .strip_prefix("--output=")
            .is_some_and(|value| value.eq_ignore_ascii_case("json"))
    })
}

fn format_duration(duration: Duration) -> String {
    let total_millis = duration.as_millis();
    if total_millis < 1_000 {
        return format!("{total_millis}ms");
    }

    let total_seconds = total_millis / 1_000;
    if total_seconds < 60 {
        let fractional_millis = total_millis % 1_000;
        if fractional_millis == 0 {
            return format!("{total_seconds}s");
        }

        let mut value = format!("{total_seconds}.{fractional_millis:03}");
        while value.ends_with('0') {
            value.pop();
        }
        return format!("{value}s");
    }

    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    if total_minutes < 60 {
        return format!("{total_minutes}m {seconds:02}s");
    }

    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;
    format!("{hours}h {minutes:02}m {seconds:02}s")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_subsecond_duration() {
        assert_eq!(format_duration(Duration::from_millis(842)), "842ms");
    }

    #[test]
    fn formats_fractional_seconds_without_trailing_zeroes() {
        assert_eq!(format_duration(Duration::from_millis(1_250)), "1.25s");
        assert_eq!(format_duration(Duration::from_millis(1_005)), "1.005s");
    }

    #[test]
    fn formats_minutes_like_codex_cli() {
        assert_eq!(format_duration(Duration::from_secs(654)), "10m 54s");
    }

    #[test]
    fn formats_hours() {
        assert_eq!(format_duration(Duration::from_secs(3_723)), "1h 02m 03s");
    }

    #[test]
    fn embedded_build_identifier_is_present() {
        assert!(!env!("TAKOKIT_BUILD_ID").trim().is_empty());
    }

    #[test]
    fn runtime_endpoint_contract_is_json_serializable() {
        let config = RuntimeConfig {
            host: "127.0.0.1".into(),
            port: 5050,
            storage_root: std::path::PathBuf::from("/tmp/takokit"),
        };
        let value = serde_json::to_value(config).unwrap();
        assert_eq!(value["host"], "127.0.0.1");
        assert_eq!(value["port"], 5050);
        assert_eq!(value["storage_root"], "/tmp/takokit");
    }
}
