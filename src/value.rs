//! Typed views of the `time` and `band` attributes.

use crate::{Error, Result};

/// A date and time with a fixed UTC offset (QxSL's `ZonedDateTime`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    /// Offset from UTC in minutes (`540` for JST).
    pub offset: i32,
}

/// JST, the zone QxSL's decoders use for local times when run in Japan.
pub const JST: i32 = 540;

impl Time {
    pub fn new(ymd: (i32, u32, u32), hms: (u32, u32, u32), offset: i32) -> Self {
        Time {
            year: ymd.0,
            month: ymd.1,
            day: ymd.2,
            hour: hms.0,
            minute: hms.1,
            second: hms.2,
            offset,
        }
    }

    /// Seconds since 1970-01-01T00:00Z.
    pub fn unix(&self) -> i64 {
        let days = days_from_civil(self.year, self.month, self.day);
        let secs = (self.hour * 3600 + self.minute * 60 + self.second) as i64;
        days * 86400 + secs - self.offset as i64 * 60
    }

    pub fn from_unix(secs: i64, offset: i32) -> Self {
        let local = secs + offset as i64 * 60;
        let (y, m, d) = civil_from_days(local.div_euclid(86400));
        let s = local.rem_euclid(86400) as u32;
        Time::new((y, m, d), (s / 3600, s / 60 % 60, s % 60), offset)
    }

    /// The same instant at another offset (`withZoneSameInstant`).
    pub fn at_offset(&self, offset: i32) -> Self {
        Time::from_unix(self.unix(), offset)
    }

    /// The current time at the offset.
    pub fn now(offset: i32) -> Self {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Time::from_unix(secs, offset)
    }

    /// ISO 8601 as `ISO_ZONED_DATE_TIME` writes it: seconds always, `Z` for UTC.
    pub fn to_iso(&self) -> String {
        let zone = if self.offset == 0 {
            "Z".to_string()
        } else {
            let a = self.offset.abs();
            let sign = if self.offset < 0 { '-' } else { '+' };
            format!("{sign}{:02}:{:02}", a / 60, a % 60)
        };
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{zone}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// Parse `yyyy-MM-ddTHH:mm[:ss[.f]]` followed by `Z`, `+HH:MM`, `+HHMM`, `+HH` (or `-`),
    /// `UTC`, `GMT` or `JST`, optionally with a `[region]` suffix.
    pub fn parse_iso(text: &str) -> Result<Self> {
        let bad = || Error::new(format!("malformed time: {text}"));
        let text = text.trim();
        let text = match text.find('[') {
            Some(p) => &text[..p],
            None => text,
        };
        let (date, rest) = text.split_once('T').ok_or_else(bad)?;
        let mut d = date.split('-');
        let year: i32 = num(d.next(), bad)?;
        let month: u32 = num(d.next(), bad)?;
        let day: u32 = num(d.next(), bad)?;
        let split = rest
            .find(|c: char| c == 'Z' || c == '+' || c == '-' || c.is_ascii_alphabetic())
            .unwrap_or(rest.len());
        let (clock, zone) = rest.split_at(split);
        let mut c = clock.split(':');
        let hour: u32 = num(c.next(), bad)?;
        let minute: u32 = num(c.next(), bad)?;
        let second: u32 = match c.next() {
            Some(s) => num(Some(s.split('.').next().unwrap_or("")), bad)?,
            None => 0,
        };
        let offset = match zone {
            "Z" | "UTC" | "GMT" => 0,
            "JST" => JST,
            z if z.starts_with('+') || z.starts_with('-') => {
                let sign = if z.starts_with('-') { -1 } else { 1 };
                let digits: String = z[1..].chars().filter(|c| *c != ':').collect();
                let (h, m) = match digits.len() {
                    2 => (&digits[..2], "0"),
                    4 => (&digits[..2], &digits[2..]),
                    _ => return Err(bad()),
                };
                sign * (num::<i32>(Some(h), bad)? * 60 + num::<i32>(Some(m), bad)?)
            }
            _ => return Err(bad()),
        };
        let t = Time::new((year, month, day), (hour, minute, second), offset);
        if !(1..=12).contains(&month) || day == 0 || day > 31 || hour > 23 || minute > 59 {
            return Err(bad());
        }
        Ok(t)
    }
}

fn num<T: std::str::FromStr>(s: Option<&str>, bad: impl Fn() -> Error) -> Result<T> {
    s.and_then(|s| s.trim().parse().ok()).ok_or_else(bad)
}

// Howard Hinnant's algorithms for the proleptic Gregorian calendar.
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y as i32, m, d)
}

/// A frequency, kept exactly in hertz (QxSL's `Band`, a decimal number of kHz).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Band(pub u64);

impl Band {
    pub fn from_khz(khz: u64) -> Self {
        Band(khz * 1000)
    }

    /// Parse as `BandFactory` does: a decimal number followed by `Hz`, `kHz`, `MHz`,
    /// `GHz` or nothing (kHz).
    pub fn parse(text: &str) -> Result<Self> {
        let bad = || Error::new(format!("malformed band: {text}"));
        let t = text.trim();
        let (digits, exp) = if let Some(d) = t.strip_suffix("kHz") {
            (d, 3)
        } else if let Some(d) = t.strip_suffix("MHz") {
            (d, 6)
        } else if let Some(d) = t.strip_suffix("GHz") {
            (d, 9)
        } else if let Some(d) = t.strip_suffix("Hz") {
            (d, 0)
        } else {
            (t, 3)
        };
        let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
        if int.is_empty() && frac.is_empty()
            || !int.chars().chain(frac.chars()).all(|c| c.is_ascii_digit())
            || frac.len() > exp
        {
            return Err(bad());
        }
        let mut s = format!("{int}{frac}");
        s.push_str(&"0".repeat(exp - frac.len()));
        s.parse::<u64>().map(Band).map_err(|_| bad())
    }

    /// The frequency in kHz, as `BandFactory` encodes it (`14000`, `1900`, `135.7`).
    pub fn khz(&self) -> String {
        decimal(self.0, 3)
    }

    /// `toDecimalString(3)`: the frequency in MHz (`14`, `1.9`, `1200`).
    pub fn mhz(&self) -> String {
        decimal(self.0, 6)
    }

    /// `toString`: `135kHz`, `14MHz`, `1200MHz`, `10GHz`.
    pub fn label(&self) -> String {
        let khz = self.0 as f64 / 1000.0;
        if khz > 1e6 {
            format!("{}GHz", decimal(self.0, 9))
        } else if khz > 1e3 {
            format!("{}MHz", self.mhz())
        } else {
            format!("{}kHz", self.khz())
        }
    }
}

fn decimal(hz: u64, exp: u32) -> String {
    let unit = 10u64.pow(exp);
    let int = hz / unit;
    let frac = hz % unit;
    if frac == 0 {
        return int.to_string();
    }
    let f = format!("{frac:0width$}", width = exp as usize);
    format!("{int}.{}", f.trim_end_matches('0'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_spellings() {
        assert_eq!(Band::parse("14MHz").unwrap().khz(), "14000");
        assert_eq!(Band::parse("1.9MHz").unwrap().khz(), "1900");
        assert_eq!(Band::parse("1.2GHz").unwrap().label(), "1.2GHz");
        assert_eq!(Band::parse("430MHz").unwrap().label(), "430MHz");
        assert_eq!(Band::parse("10GHz").unwrap().label(), "10GHz");
        assert_eq!(Band::parse("135").unwrap().label(), "135kHz");
        assert_eq!(Band::parse("1900").unwrap().mhz(), "1.9");
        assert!(Band::parse("abc").is_err());
    }

    #[test]
    fn time_iso() {
        let t = Time::parse_iso("2017-06-04T00:00Z").unwrap();
        assert_eq!(t.at_offset(JST).to_iso(), "2017-06-04T09:00:00+09:00");
        let u = Time::parse_iso("2017-06-04T09:00:00+09:00[Asia/Tokyo]").unwrap();
        assert_eq!(u.unix(), t.unix());
        assert_eq!(
            Time::from_unix(1496534400, 0).to_iso(),
            "2017-06-04T00:00:00Z"
        );
    }
}
