//! Rendering numbers so a narrow column cannot make them lie.
//!
//! Ratatui shrinks columns to fit, cutting the text it cannot show. For a name that is
//! obvious — `ether1-WAN` becomes `ethe`. For a number it is not: a byte count of
//! 194810240 clipped to four columns became `194`, which reads as a perfectly plausible
//! byte count. Abbreviating instead keeps the magnitude true in the width available.

/// Format a packet or item count, e.g. `1490210` as `1.49M`.
pub fn count(value: u64) -> String {
    scale(value, 1000.0, &["", "K", "M", "G", "T", "P", "E"])
}

/// Format a byte count in binary units, e.g. `194810240` as `185.8Mi`.
///
/// The `i` marks these as binary multiples, matching what RouterOS reports.
pub fn bytes(value: u64) -> String {
    scale(value, 1024.0, &["B", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei"])
}

fn scale(value: u64, step: f64, units: &[&str]) -> String {
    if value < step as u64 {
        return format!("{value}{}", units[0]);
    }

    let mut scaled = value as f64;
    let mut unit = 0;
    while scaled >= step && unit + 1 < units.len() {
        scaled /= step;
        unit += 1;
    }

    // Three significant figures, and units all the way to exa, keep every result inside
    // six characters for any u64 — which is what lets the column be narrow enough to stop
    // being squeezed in the first place.
    if scaled < 10.0 {
        format!("{scaled:.2}{}", units[unit])
    } else if scaled < 100.0 {
        format!("{scaled:.1}{}", units[unit])
    } else {
        format!("{scaled:.0}{}", units[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The values that were being truncated in the firewall and interface tables.
    #[test]
    fn the_counts_that_used_to_be_clipped_now_fit() {
        assert_eq!(count(1_490_210), "1.49M");
        assert_eq!(count(194_810_240), "195M");
        assert_eq!(bytes(194_810_240), "186Mi");

        // Six characters is the budget the narrow columns are sized for.
        for value in [0, 1, 999, 1_000, 1_048_576, u64::MAX] {
            assert!(count(value).len() <= 6, "count({value}) = {}", count(value));
            assert!(bytes(value).len() <= 6, "bytes({value}) = {}", bytes(value));
        }
    }

    /// Small numbers are exact: abbreviating `42` to `42.0` would lose nothing but read
    /// worse, and a packet count of 0 should say 0.
    #[test]
    fn small_values_are_left_alone() {
        assert_eq!(count(0), "0");
        assert_eq!(count(42), "42");
        assert_eq!(count(999), "999");
        assert_eq!(bytes(0), "0B");
        assert_eq!(bytes(512), "512B");
    }

    #[test]
    fn units_step_at_the_right_boundaries() {
        assert_eq!(count(1_000), "1.00K");
        assert_eq!(count(999_999), "1000K");
        assert_eq!(count(1_000_000), "1.00M");

        assert_eq!(bytes(1_024), "1.00Ki");
        assert_eq!(bytes(1_048_576), "1.00Mi");
        assert_eq!(bytes(1_073_741_824), "1.00Gi");
    }

    /// Whatever it prints must still be the right order of magnitude — that is the whole
    /// point compared with truncation.
    #[test]
    fn the_magnitude_survives() {
        for (value, expected_unit) in [
            (999u64, ""),
            (1_500, "K"),
            (1_500_000, "M"),
            (1_500_000_000, "G"),
            (1_500_000_000_000, "T"),
        ] {
            let out = count(value);
            assert!(
                out.ends_with(expected_unit),
                "count({value}) = {out}, expected unit {expected_unit:?}"
            );
        }
    }
}
