//! time handles parsing of xsd:dateTime.

use std::io::Read;
use chrono::Datelike;
/// format: [-]CCYY-MM-DDThh:mm:ss[Z|(+|-)hh:mm]
#[cfg(feature = "use-serde")]
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Iso8601, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::errors::GpxResult;
use crate::parser::{string, Context};

#[derive(Debug, Clone, Copy, Eq, Ord, PartialOrd, PartialEq, Hash)]
#[cfg_attr(feature = "use-serde", derive(Serialize, Deserialize))]
pub struct Time(OffsetDateTime);

impl Time {
    /// Render time in ISO 8601 format
    pub fn format(&self) -> GpxResult<String> {
        self.0.format(&Iso8601::DEFAULT).map_err(From::from)
    }
}

impl From<OffsetDateTime> for Time {
    fn from(t: OffsetDateTime) -> Self {
        Time(t)
    }
}

impl From<Time> for OffsetDateTime {
    fn from(t: Time) -> Self {
        t.0
    }
}


fn parse_with_chrono(input: &str) -> Option<OffsetDateTime> {
    let mut s = input.trim().to_string();

    if let Some(idx) = s.rfind(" +") { if idx >= 19 { s.remove(idx); } }
    if let Some(idx) = s.rfind(" -") { if idx >= 19 { s.remove(idx); } }

    if s.len() >= 10 {
        let date = s[..10].replace('/', "-");
        s.replace_range(0..10, &date);
    }

    const WITH_OFFSET: &[&str] = &[
        "%Y-%m-%d %H:%M:%S %z",
        "%Y-%m-%dT%H:%M:%S%.f%z",
        "%Y-%m-%dT%H:%M:%S%z",
    ];

    const UTC: &[&str] = &[
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
    ];

    for fmt in WITH_OFFSET {
        if let Ok(dt) = chrono::DateTime::parse_from_str(&s, fmt) {
            if dt.year() < 0 {
                continue;
            }
            if let Some(nanos) = dt.timestamp_nanos_opt() {
                return OffsetDateTime::from_unix_timestamp_nanos(nanos as i128).ok();
            }
        }
    }

    for fmt in UTC {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, fmt) {
            if dt.year() < 0 {
                continue;
            }
            let utc = dt.and_utc();
            return OffsetDateTime::from_unix_timestamp(utc.timestamp())
                .ok()
                .map(|t| t.to_offset(UtcOffset::UTC));
        }
    }

    None
}


/// consume consumes an element as a time.
pub fn consume<R: Read>(context: &mut Context<R>) -> GpxResult<Time> {
    let time_str = string::consume(context, "time", false)?;

    let strict = OffsetDateTime::parse(&time_str, &Iso8601::PARSING)
        .or_else(|_| {
            PrimitiveDateTime::parse(&time_str, &Iso8601::PARSING)
                .map(PrimitiveDateTime::assume_utc)
        });

    if let Ok(t) = strict {
        return Ok(t.to_offset(UtcOffset::UTC).into());
    }

    if let Some(t) = parse_with_chrono(&time_str) {
        return Ok(t.to_offset(UtcOffset::UTC).into());
    }

    Err(strict.err().unwrap().into())
}

#[cfg(test)]
mod tests {
    use crate::GpxVersion;

    use super::consume;

    #[test]
    fn consume_time() {
        let result = consume!("<time>1996-12-19T16:39:57-08:00</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());

        // The following examples are taken from the xsd:dateTime examples.
        let result = consume!("<time>2001-10-26T21:32:52</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());

        let result = consume!("<time>2001-10-26T21:32:52+02:00</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());

        let result = consume!("<time>2001-10-26T19:32:52Z</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());

        let result = consume!("<time>2001-10-26T19:32:52+00:00</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());

        let result = consume!("<time>2001-10-26T21:32:52.12679</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());

        let result = consume!("<time>2001-10-26T21:32</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());

        // lenient cases
        let result = consume!("<time>2025-07-02 18:07:28 +0000</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());

        let result = consume!("<time>2025/07/02 18:07:28</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());

        // invalid
        let result = consume!("<time>2001-10-26</time>", GpxVersion::Gpx11);
        assert!(result.is_err());

        let result = consume!("<time>2001-10-26T25:32:52+02:00</time>", GpxVersion::Gpx11);
        assert!(result.is_err());

        let result = consume!("<time>01-10-26T21:32</time>", GpxVersion::Gpx11);
        assert!(result.is_err());

        // TODO we currently don't allow for negative years although the standard demands it
        //  see https://www.w3.org/TR/xmlschema-2/#dateTime
        let result = consume!("<time>-2001-10-26T21:32:52</time>", GpxVersion::Gpx11);
        assert!(result.is_err());

        // https://github.com/georust/gpx/issues/77
        let result = consume!("<time>2021-10-10T09:55:20.952</time>", GpxVersion::Gpx11);
        assert!(result.is_ok());
    }
}
