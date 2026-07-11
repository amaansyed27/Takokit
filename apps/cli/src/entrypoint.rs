use std::time::{Duration, Instant};

pub async fn run() -> anyhow::Result<()> {
    let started = Instant::now();
    let show_timing = !std::env::args_os().any(|argument| argument == "--daemon-child");
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

        return format!("{total_seconds}.{fractional_millis:03}s")
            .trim_end_matches('0')
            .trim_end_matches('s')
            .to_string()
            + "s";
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
}
