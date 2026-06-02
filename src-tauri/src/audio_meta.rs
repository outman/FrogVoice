/// Parse duration from ffmpeg `-i` stderr output
pub fn parse_duration(ffmpeg_output: &str) -> Option<f64> {
    for line in ffmpeg_output.lines() {
        if let Some(start) = line.find("Duration: ") {
            let duration_part = &line[start + 10..];
            let end = duration_part.find(',').unwrap_or(duration_part.len());
            let duration_str = duration_part[..end].trim();
            let parts: Vec<&str> = duration_str.split(':').collect();
            if parts.len() == 3 {
                let hours: f64 = parts[0].parse().ok()?;
                let minutes: f64 = parts[1].parse().ok()?;
                let seconds: f64 = parts[2].parse().ok()?;
                return Some(hours * 3600.0 + minutes * 60.0 + seconds);
            }
        }
    }
    None
}

/// Format duration in seconds to "M:SS" display string
pub fn format_duration(secs: f64) -> String {
    let total_secs = secs as u64;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{}:{:02}", minutes, seconds)
}

/// Parse conversion progress from ffmpeg stderr line
/// Returns progress as 0.0-1.0 fraction
pub fn parse_progress(line: &str, total_duration_secs: f64) -> Option<f64> {
    if total_duration_secs <= 0.0 {
        return None;
    }
    if let Some(start) = line.find("time=") {
        let time_part = &line[start + 5..];
        let end = time_part.find(' ').unwrap_or(time_part.len());
        let time_str = &time_part[..end];
        // ffmpeg outputs negative time (e.g. -00:00:00.00) when no progress yet
        if time_str.starts_with('-') {
            return None;
        }
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() == 3 {
            let hours: f64 = parts[0].parse().ok()?;
            let minutes: f64 = parts[1].parse().ok()?;
            let seconds: f64 = parts[2].parse().ok()?;
            let current = hours * 3600.0 + minutes * 60.0 + seconds;
            return Some((current / total_duration_secs).clamp(0.0, 1.0));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_standard() {
        let output = "ffmpeg version 6.0 Copyright (c) 2000-2023\n  Duration: 00:04:32.15, start: 0.000000, bitrate: 128 kb/s\n";
        assert_eq!(parse_duration(output), Some(272.15));
    }

    #[test]
    fn test_parse_duration_short() {
        let output = "  Duration: 00:00:05.00, start: 0.000000\n";
        assert_eq!(parse_duration(output), Some(5.0));
    }

    #[test]
    fn test_parse_duration_not_found() {
        let output = "ffmpeg version 6.0\nNo such file or directory\n";
        assert_eq!(parse_duration(output), None);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(272.15), "4:32");
        assert_eq!(format_duration(65.0), "1:05");
        assert_eq!(format_duration(5.0), "0:05");
        assert_eq!(format_duration(0.0), "0:00");
    }

    #[test]
    fn test_parse_progress() {
        let line = "size=    1024kB time=00:02:15.50 bitrate=  61.8kbits/s speed=  128x";
        assert_eq!(parse_progress(line, 272.15), Some(135.5 / 272.15));
    }

    #[test]
    fn test_parse_progress_no_time() {
        let line = "size=       0kB time=-00:00:00.00 bitrate=   0.0kbits/s";
        assert_eq!(parse_progress(line, 272.15), None);
    }

    #[test]
    fn test_parse_progress_zero_duration() {
        let line = "size=    1024kB time=00:00:05.00 bitrate=  61.8kbits/s";
        assert_eq!(parse_progress(line, 0.0), None);
    }
}
