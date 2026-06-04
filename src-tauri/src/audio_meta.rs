/// Format duration in seconds to "M:SS" display string
pub fn format_duration(secs: f64) -> String {
    let total_secs = secs as u64;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{}:{:02}", minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(272.15), "4:32");
        assert_eq!(format_duration(65.0), "1:05");
        assert_eq!(format_duration(5.0), "0:05");
        assert_eq!(format_duration(0.0), "0:00");
    }
}
