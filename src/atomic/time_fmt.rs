// SPDX-License-Identifier: MIT OR Apache-2.0

//! UTC timestamp helpers for backup names and NDJSON.

pub(crate) fn utc_timestamp_formatted() -> String {
    use std::time::SystemTime;
    // duration_since fails only if system clock precedes UNIX epoch — defaults to 1970-01-01
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    let (year, month, day, hour, min, sec) = epoch_to_utc(secs);
    format!("{year:04}{month:02}{day:02}_{hour:02}{min:02}{sec:02}_{millis:03}")
}

/// Return the current UTC time as an RFC 3339 string (e.g. `2024-01-15T14:30:22Z`).
pub fn rfc3339_now() -> String {
    use std::time::SystemTime;
    // duration_since fails only if system clock precedes UNIX epoch — defaults to 1970-01-01
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d, h, min, sec) = epoch_to_utc(secs);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{sec:02}Z")
}

/// Convert Unix epoch seconds to `(year, month, day, hour, min, sec)` UTC.
pub fn epoch_to_utc(epoch: u64) -> (u64, u64, u64, u64, u64, u64) {
    let sec_of_day = epoch % 86400;
    let hour = sec_of_day / 3600;
    let min = (sec_of_day % 3600) / 60;
    let sec = sec_of_day % 60;

    let mut days = (epoch / 86400) as i64;
    days += 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = (days - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y as u64, m, d, hour, min, sec)
}
