//! Pure formatting of a Unix millisecond timestamp as an ISO-8601 UTC string.

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let yoe_i = yoe as i64;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 {
        yoe_i + era * 400 + 1
    } else {
        yoe_i + era * 400
    };
    (year, month, day)
}

/// Formats a Unix timestamp in milliseconds as `YYYY-MM-DDTHH:MM:SS.sssZ` (UTC).
pub fn iso_from_millis(millis: u64) -> String {
    let ms = millis % 1000;
    let total_seconds = millis / 1000;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let total_hours = total_minutes / 60;
    let hours = total_hours % 24;
    let days = (total_hours / 24) as i64;

    let (year, month, day) = civil_from_days(days);

    format!(
        "{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{ms:03}Z"
    )
}

#[cfg(test)]
mod tests {
    use super::iso_from_millis;

    #[test]
    fn epoch_is_zero() {
        assert_eq!(iso_from_millis(0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn known_date_in_2026() {
        assert_eq!(iso_from_millis(1_784_764_800_000), "2026-07-23T00:00:00.000Z");
    }

    #[test]
    fn sub_second_remainder() {
        assert_eq!(iso_from_millis(1_234), "1970-01-01T00:00:01.234Z");
    }

    #[test]
    fn one_full_day() {
        assert_eq!(iso_from_millis(86_400_000), "1970-01-02T00:00:00.000Z");
    }

    #[test]
    fn end_of_february_leap_year() {
        // 2000-02-29T00:00:00.000Z
        assert_eq!(iso_from_millis(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }

    #[test]
    fn year_boundary() {
        // 1999-12-31T23:59:59.999Z
        assert_eq!(iso_from_millis(946_684_799_999), "1999-12-31T23:59:59.999Z");
    }

    #[test]
    fn padding_for_single_digit_fields() {
        // 2001-02-03T04:05:06.007Z
        assert_eq!(iso_from_millis(981_173_106_007), "2001-02-03T04:05:06.007Z");
    }

    #[test]
    fn far_future_four_digit_year() {
        assert_eq!(iso_from_millis(253_402_300_799_000), "9999-12-31T23:59:59.000Z");
    }
}
