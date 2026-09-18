//! Rendering UTC instants in the viewer's local timezone.
//!
//! Every absolute instant this UI shows arrives as UTC — `container.created` and
//! `container.started_at` as unix seconds, log lines as an RFC 3339 prefix the Docker
//! daemon stamped on. The viewer's offset is only knowable in the browser, so the whole
//! localisation happens here; nothing about the zone ever goes over the wire.
//!
//! The offset is derived ONCE and cached. Both readings come from `js_sys::Date`, which
//! is a wasm-bindgen import rather than `eval` — it cannot throw, so nothing in this
//! module can panic the app.

use std::cell::Cell;

use rust_extensions::date_time::{
    DateTimeAsMicroseconds, DateTimeAsMicrosecondsWithTimeZone, TimeZone,
};

thread_local! {
    /// `wasm32-unknown-unknown` is single-threaded, so this is one plain static.
    /// A `Cell` rather than a `OnceLock` because `refresh()` re-sets it across a
    /// DST transition on a tab that has been open for days.
    static LOCAL_TZ: Cell<Option<TimeZone>> = const { Cell::new(None) };
}

/// The viewer's timezone, derived once and cached. Falls back to UTC if the browser
/// hands back something nonsensical — never panics, never returns an error.
pub fn time_zone() -> TimeZone {
    LOCAL_TZ.with(|slot| match slot.get() {
        Some(tz) => tz,
        None => {
            let tz = derive_time_zone();
            slot.set(Some(tz));
            tz
        }
    })
}

/// Re-derives the cached offset. Called from the polling loop so a tab left open across
/// a DST change follows it without a reload. Costs one `Date` allocation.
pub fn refresh() {
    LOCAL_TZ.with(|slot| slot.set(Some(derive_time_zone())));
}

/// `TimeZone::from_server_and_local_time` fed with the browser's own clock on both
/// sides: `server` is UTC now, `local` is the same instant shifted by the browser's
/// `getTimezoneOffset()`. Their difference is the offset, rounded to the nearest 15
/// minutes by rust-extensions (lossless — every real offset is a multiple of 15).
#[cfg(target_arch = "wasm32")]
fn derive_time_zone() -> TimeZone {
    let server = DateTimeAsMicroseconds::now();

    // `getTimezoneOffset()` is UTC-minus-local in minutes and positive WEST of
    // Greenwich, i.e. the sign is inverted from what a zone offset reads as.
    let js_offset = js_sys::Date::new_0().get_timezone_offset();

    // Real offsets never exceed ±14:00. Anything else (NaN, garbage) means UTC.
    if !js_offset.is_finite() || js_offset.abs() > 900.0 {
        return TimeZone::utc();
    }

    let mut local = server;
    local.add_minutes(-(js_offset as i64));

    TimeZone::from_server_and_local_time(server, local)
}

/// Host builds (`cargo test`) have no browser clock; the formatting tests below pass an
/// explicit `TimeZone`, so this only has to be a sane default.
#[cfg(not(target_arch = "wasm32"))]
fn derive_time_zone() -> TimeZone {
    TimeZone::utc()
}

/// `UTC+03:00` / `UTC-05:00` / `UTC` — the topbar chip's text, so the viewer can see
/// which zone every timestamp below is in.
pub fn zone_label() -> String {
    zone_label_of(time_zone())
}

fn zone_label_of(tz: TimeZone) -> String {
    let minutes = tz.offset_in_minutes();
    if minutes == 0 {
        return "UTC".to_string();
    }
    let sign = if minutes < 0 { '-' } else { '+' };
    let abs = minutes.abs();
    format!("UTC{}{:02}:{:02}", sign, abs / 60, abs % 60)
}

/// `YYYY-MM-DD HH:MM:SS` in the viewer's zone, from unix SECONDS.
/// Used for `container.started_at`.
pub fn local_from_unix_seconds(seconds: i64) -> String {
    local_from_micros(seconds * 1_000_000)
}

/// `YYYY-MM-DD HH:MM:SS` in the viewer's zone, from a raw Docker `created` value.
/// `From<i64>` detects the unit by magnitude, which is what `container.created` needs.
pub fn local_from_docker_epoch(value: i64) -> String {
    local_from_micros(DateTimeAsMicroseconds::from(value).unix_microseconds)
}

/// `YYYY-MM-DD HH:MM:SS` in the viewer's zone, from unix MICROSECONDS.
pub fn local_from_micros(micros: i64) -> String {
    with_zone(micros, time_zone()).to_compact_string()
}

/// The same instant as UTC — `2026-08-23T09:14:07.000000Z`. Goes in a `title=` tooltip
/// beside a localised value so the original is one hover away.
pub fn utc_from_docker_epoch(value: i64) -> String {
    DateTimeAsMicroseconds::from(value).to_rfc3339_utc()
}

/// The same, from unix SECONDS.
pub fn utc_from_unix_seconds(seconds: i64) -> String {
    DateTimeAsMicroseconds::new(seconds * 1_000_000).to_rfc3339_utc()
}

/// Rewrites one Docker log line so its leading UTC timestamp reads in the viewer's zone.
///
/// Docker stamps `2026-08-23T11:12:23.565640624Z ` (RFC 3339, always UTC, always a `Z`,
/// today always a 9-digit fraction) in front of every line because both log requests set
/// `timestamps=1`, and nothing between the daemon and here rewrites it. The output keeps
/// millisecond precision — `2026-08-23 14:12:23.565 <message>`.
///
/// Returns the line UNCHANGED when there is no recognisable prefix. That covers the
/// synthetic `── reconnected ──` markers (whose box-drawing chars are multi-byte and
/// would be mangled by any blind slice), TTY-mode raw output, and anything a future
/// injection adds. It is shape-gated rather than `tp`-gated on purpose, so an
/// untimestamped line is safe by construction.
pub fn localize_log_line(line: &str) -> String {
    match split_docker_timestamp(line) {
        Some((ts, rest)) => match DateTimeAsMicroseconds::parse_iso_string(ts) {
            Some(utc) => {
                let local =
                    with_zone(utc.unix_microseconds, time_zone()).to_local_date_time_struct();
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03} {}",
                    local.year,
                    local.month,
                    local.day,
                    local.time.hour,
                    local.time.min,
                    local.time.sec,
                    local.time.micros / 1000,
                    rest
                )
            }
            // Shape passed but the date is impossible (month 13, day 32). Leave it be.
            None => line.to_string(),
        },
        None => line.to_string(),
    }
}

/// Splits `"<rfc3339> <message>"` into its two halves, or `None` when the line does not
/// start with something that is unambiguously an RFC 3339 instant.
///
/// The separator is FOUND, not assumed at a fixed index: today Docker emits a padded
/// 9-digit fraction, but a driver that trims trailing zeros would shift every byte after
/// the dot, and a hardcoded width would then silently mangle the message. Every byte the
/// guard checks is ASCII, so both slices are always on a char boundary — the multi-byte
/// marker lines are rejected by the guard long before that matters. `parse_iso_string`
/// itself validates NO separators (it indexes `src[5..7]` blind), which is exactly why
/// this guard has to exist.
fn split_docker_timestamp(line: &str) -> Option<(&str, &str)> {
    let b = line.as_bytes();

    let sp = b.iter().position(|c| *c == b' ')?;
    // `2026-08-23T11:12:23Z` is 20; the padded-nanos form is 30. Anything outside that
    // band is not a Docker stamp.
    if sp < 20 || sp > 40 {
        return None;
    }

    let ts = &b[..sp];
    if ts[4] != b'-' || ts[7] != b'-' || ts[10] != b'T' || ts[13] != b':' || ts[16] != b':' {
        return None;
    }
    if !ts[..4].iter().all(u8::is_ascii_digit)
        || !ts[5..7].iter().all(u8::is_ascii_digit)
        || !ts[8..10].iter().all(u8::is_ascii_digit)
        || !ts[11..13].iter().all(u8::is_ascii_digit)
        || !ts[14..16].iter().all(u8::is_ascii_digit)
        || !ts[17..19].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    // Docker always ends the stamp with `Z`; accept a numeric offset too rather than
    // dropping the line on a driver that emits one.
    let last = *ts.last()?;
    if last != b'Z' && !last.is_ascii_digit() {
        return None;
    }

    // The remainder keeps its leading whitespace — pretty-printed JSON and stack traces
    // are indented, and trimming here would visibly wreck the log view.
    Some((&line[..sp], &line[sp + 1..]))
}

fn with_zone(micros: i64, tz: TimeZone) -> DateTimeAsMicrosecondsWithTimeZone {
    DateTimeAsMicrosecondsWithTimeZone::new(DateTimeAsMicroseconds::new(micros), tz)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The browser-clock read needs a real `Date`, so `derive_time_zone` is exercised in
    // the browser. These pin the two halves that can be tested on the host: the shape
    // guard, and the offset arithmetic.

    fn micros(utc: &str) -> i64 {
        DateTimeAsMicroseconds::parse_iso_string(utc)
            .unwrap()
            .unix_microseconds
    }

    #[test]
    fn an_instant_renders_in_the_given_offset() {
        let utc = micros("2021-04-25T17:30:03.000000Z");
        assert_eq!(
            with_zone(utc, TimeZone::from_minutes(60)).to_compact_string(),
            "2021-04-25 18:30:03"
        );
    }

    #[test]
    fn a_negative_offset_rolls_back_across_midnight() {
        let utc = micros("2021-04-25T00:30:00.000000Z");
        assert_eq!(
            with_zone(utc, TimeZone::from_minutes(-120)).to_compact_string(),
            "2021-04-24 22:30:00"
        );
    }

    #[test]
    fn zone_labels_read_as_utc_offsets() {
        assert_eq!(zone_label_of(TimeZone::utc()), "UTC");
        assert_eq!(zone_label_of(TimeZone::from_minutes(180)), "UTC+03:00");
        assert_eq!(zone_label_of(TimeZone::from_minutes(-300)), "UTC-05:00");
        assert_eq!(zone_label_of(TimeZone::from_minutes(345)), "UTC+05:45");
    }

    #[test]
    fn a_docker_line_splits_at_the_separator() {
        let line = "2026-08-23T11:12:23.565640624Z     \"active-data\": 21362,";
        let (ts, rest) = split_docker_timestamp(line).unwrap();
        assert_eq!(ts, "2026-08-23T11:12:23.565640624Z");
        // Indentation survives — this is pretty-printed JSON.
        assert_eq!(rest, "    \"active-data\": 21362,");
    }

    #[test]
    fn nine_digit_fractions_parse_to_microseconds() {
        let utc =
            DateTimeAsMicroseconds::parse_iso_string("2026-08-23T11:12:23.565640624Z").unwrap();
        assert_eq!(utc.to_rfc3339_utc(), "2026-08-23T11:12:23.565640Z");
    }

    #[test]
    fn untimestamped_lines_pass_through_untouched() {
        for line in [
            "── reconnected ──",
            "── disconnected (open failed: Err) ──",
            "",
            " ",
            "plain message with a space",
            "2026-08-23T11:12:23.565640624Z", // stamp with no message and no separator
            "\u{1b}[32mINFO\u{1b}[0m starting",
        ] {
            assert_eq!(localize_log_line(line), line, "mangled: {line}");
        }
    }

    #[test]
    fn a_short_second_precision_stamp_still_splits() {
        let (ts, rest) = split_docker_timestamp("2026-08-23T11:12:23Z hello").unwrap();
        assert_eq!(ts, "2026-08-23T11:12:23Z");
        assert_eq!(rest, "hello");
    }
}
