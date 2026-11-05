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

/// Convert a UTC civil date and time back to Unix epoch seconds.
///
/// The inverse of [`epoch_to_utc`], with the same Howard Hinnant civil
/// algorithm: a backup name carries the moment it was created, and recovering
/// that moment from the name is what makes pruning independent of the mtime a
/// copy tool may have rewritten.
pub fn utc_to_epoch(year: i64, month: i64, day: i64, hour: u64, min: u64, sec: u64) -> u64 {
    // Days from 1970-01-01 to the civil date, valid for any proleptic Gregorian
    // date. A month before March belongs to the previous civil year.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = ((month + 9) % 12) as u64;
    let doy = (153 * mp + 2) / 5 + (day as u64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe as i64 - 719_468;
    days as u64 * crate::constants::SECS_PER_DAY
        + hour * crate::constants::SECS_PER_HOUR
        + min * crate::constants::SECS_PER_MINUTE
        + sec
}

/// Convert Unix epoch seconds to `(year, month, day, hour, min, sec)` UTC.
pub fn epoch_to_utc(epoch: u64) -> (u64, u64, u64, u64, u64, u64) {
    use crate::constants as C;

    let sec_of_day = epoch % C::SECS_PER_DAY;
    let hour = sec_of_day / C::SECS_PER_HOUR;
    let min = (sec_of_day % C::SECS_PER_HOUR) / C::SECS_PER_MINUTE;
    let sec = sec_of_day % C::SECS_PER_MINUTE;

    // Howard Hinnant's civil-from-days; the month terms below (153, 5, 2, 9,
    // 10, 3) map the March-based month index and belong to the algorithm.
    let era_last_day = C::CIVIL_DAYS_PER_ERA - 1;
    let mut days = (epoch / C::SECS_PER_DAY) as i64;
    days += C::CIVIL_EPOCH_SHIFT_DAYS;
    let era = if days >= 0 { days } else { days - era_last_day } / C::CIVIL_DAYS_PER_ERA;
    let doe = (days - era * C::CIVIL_DAYS_PER_ERA) as u64;
    let yoe = (doe - doe / C::CIVIL_DAYS_PER_FOUR_YEARS + doe / C::CIVIL_DAYS_PER_CENTURY
        - doe / era_last_day as u64)
        / C::CIVIL_DAYS_PER_YEAR;
    let y = (yoe as i64) + era * C::CIVIL_YEARS_PER_ERA;
    let doy = doe - (C::CIVIL_DAYS_PER_YEAR * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y as u64, m, d, hour, min, sec)
}
