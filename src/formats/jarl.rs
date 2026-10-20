//! JARL electronic log R2.1, QxSL's `jarl`.
//!
//! ```text
//! DATE (JST) TIME   BAND MODE  CALLSIGN      SENTNo      RCVDNo      Mlt    Pts
//! 2017-06-04 09:00   14  CW    QP3GES        599 100110  599 26      -        1
//! ```
//!
//! Fields are separated by white space; times are local; the band is in MHz and
//! `-` marks a missing multiplier.
//!
//! As zLog's `FormatELogStd`, an invalid contact (`invalid` set to `true`) has `X `
//! before its date; the multiplier column holds `mul1` only when `newmult` is not
//! `false` (zLog writes it only for a new multiplier), and the points column holds
//! `points` (1 without it). On reading, `X` sets `invalid`, a multiplier sets `mul1`
//! and `newmult` to `true`, and the points set `points`.
//!
//! The call and the numbers are never cut: a value as long as its column or longer
//! is written whole and followed by one space (zLog's `FillRight2`), where QxSL cuts
//! it.
//!
//! An empty RST or number leaves a blank column, and since the fields are read at
//! white space the following fields shift on reading; zLog's format has the same
//! ambiguity. Such a line is read as far as it goes: a field that does not parse
//! where it lands (an RST that is not an integer) is skipped, rather than failing
//! the whole file.

use crate::formats::{local_time, set_band, set_rstq, set_text, some, zlog_band, zlog_band_text};
use crate::model::{Item, QName};
use crate::text::{decode_sjis, encode_sjis, fill_tail, lines, pad_tail, right, NEWLINE};
use crate::value::JST;
use crate::{Error, Format, Result, Time};

/// The column header of the log (also of the SWL log).
pub const HEADER: &str =
    "DATE (JST) TIME   BAND MODE  CALLSIGN      SENTNo      RCVDNo      Mlt    Pts";

pub struct Jarl {
    /// UTC offset of the times in the file, in minutes (JST in JARL's rules).
    pub offset: i32,
}

impl Default for Jarl {
    fn default() -> Self {
        Jarl { offset: JST }
    }
}

/// Lines after the header, split at white space; `#CHECKLOG` lines and blank lines
/// are skipped.
pub(crate) fn records(text: &str) -> Vec<Vec<&str>> {
    let mut out = Vec::new();
    let mut head = true;
    for line in lines(text) {
        if head && (line.trim().is_empty() || line.starts_with("DATE") || line.starts_with("----"))
        {
            continue;
        }
        head = false;
        let v: Vec<&str> = line.split_whitespace().collect();
        if v.is_empty() || v.get(1) == Some(&"#CHECKLOG") {
            continue;
        }
        out.push(v);
    }
    out
}

/// `uuuu-MM-dd HH:mm` at the offset.
pub(crate) fn time(date: &str, hm: &str, offset: i32) -> Result<Time> {
    Time::parse_iso(&format!("{date}T{hm}Z"))
        .map(|t| Time { offset, ..t })
        .map_err(|_| Error::new(format!("malformed time: {date} {hm}")))
}

pub(crate) fn set_mul1(item: &mut Item, text: &str) {
    if text != "-" {
        set_text(&mut item.attrs, "mul1", text);
    }
}

pub(crate) fn date_time(item: &Item, offset: i32) -> String {
    local_time(item, offset)
        .map(|t| {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                t.year, t.month, t.day, t.hour, t.minute
            )
        })
        .unwrap_or_else(|| " ".repeat(16))
}

/// `PrintEncoder.empty`: a blank column starts with `-`.
pub(crate) fn empty(text: String) -> String {
    if text.trim().is_empty() {
        let rest: String = text.chars().skip(1).collect();
        format!("-{rest}")
    } else {
        text
    }
}

impl Format for Jarl {
    fn kind(&self) -> &'static str {
        "jarl"
    }

    fn name(&self) -> &'static str {
        "JARL LOG (R2.1)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txt"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let text = decode_sjis(bytes);
        let mut items = Vec::new();
        for mut v in records(&text) {
            let mut item = Item::default();
            if v.first() == Some(&"X") {
                v.remove(0);
                item.attrs.set(QName::qxsl("invalid"), "true");
            }
            if v.len() > 1 {
                item.attrs
                    .set(QName::qxsl("time"), time(v[0], v[1], self.offset)?.to_iso());
            }
            if let Some(b) = v.get(2) {
                set_band(&mut item.attrs, &zlog_band_text(b))?;
            }
            if let Some(m) = v.get(3) {
                set_text(&mut item.attrs, "mode", m);
            }
            if let Some(c) = v.get(4) {
                set_text(&mut item.rcvd, "call", c);
            }
            if let Some(r) = v.get(5) {
                // A malformed RST (a shifted line, see the module) is skipped.
                let _ = set_rstq(&mut item.sent, r);
            }
            if let Some(c) = v.get(6) {
                set_text(&mut item.sent, "code", c);
            }
            if let Some(r) = v.get(7) {
                // A malformed RST (a shifted line, see the module) is skipped.
                let _ = set_rstq(&mut item.rcvd, r);
            }
            if let Some(c) = v.get(8) {
                set_text(&mut item.rcvd, "code", c);
            }
            if let Some(m) = v.get(9) {
                set_mul1(&mut item, m);
                if *m != "-" {
                    item.attrs.set(QName::qxsl("newmult"), "true");
                }
            }
            if let Some(p) = v.get(10).and_then(|p| p.parse::<i64>().ok()) {
                item.attrs.set(QName::qxsl("points"), p.to_string());
            }
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = format!("{HEADER}{NEWLINE}");
        for item in items {
            if item.attrs.qxsl("invalid") == Some("true") {
                out.push_str("X ");
            }
            let mul1 = match item.attrs.qxsl("newmult") {
                Some("false") => "",
                _ => some(item, "mul1"),
            };
            let cols = [
                date_time(item, self.offset),
                right(&zlog_band(item), 5),
                pad_tail(some(item, "mode"), 5),
                fill_tail(item.rcvd.qxsl("call").unwrap_or(""), 13),
                fill_tail(item.sent.qxsl("rstq").unwrap_or(""), 3),
                fill_tail(item.sent.qxsl("code").unwrap_or(""), 7),
                fill_tail(item.rcvd.qxsl("rstq").unwrap_or(""), 3),
                fill_tail(item.rcvd.qxsl("code").unwrap_or(""), 7),
                empty(pad_tail(mul1, 6)),
            ];
            for c in cols {
                out.push_str(&c);
                out.push(' ');
            }
            out.push_str(&format!("{:>3}", item.attrs.qxsl("points").unwrap_or("1")));
            out.push_str(NEWLINE);
        }
        Ok(encode_sjis(&out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_call_and_numbers_are_written_whole() {
        let mut it = Item::default();
        it.attrs
            .set(QName::qxsl("time"), "2017-06-04T09:00:00+09:00");
        it.attrs.set(QName::qxsl("band"), "14000");
        it.attrs.set(QName::qxsl("mode"), "CW");
        it.rcvd.set(QName::qxsl("call"), "QV1DOK/MM/ABCDE");
        it.sent.set(QName::qxsl("rstq"), "599");
        it.rcvd.set(QName::qxsl("rstq"), "599");
        it.sent.set(QName::qxsl("code"), "123456789");
        it.rcvd.set(QName::qxsl("code"), "98765432");
        let text = String::from_utf8(Jarl::default().encode(&[it]).unwrap()).unwrap();
        assert!(text.contains(" QV1DOK/MM/ABCDE "), "{text}");
        assert!(text.contains(" 123456789 "), "{text}");
        assert!(text.contains(" 98765432 "), "{text}");
        let back = Jarl::default().decode(text.as_bytes()).unwrap();
        assert_eq!(back[0].rcvd.qxsl("call"), Some("QV1DOK/MM/ABCDE"));
        assert_eq!(back[0].rcvd.qxsl("code"), Some("98765432"));
    }

    #[test]
    fn a_shifted_line_does_not_fail_the_file() {
        // An empty RST shifts the fields; the number lands in the RST column.
        let text = "2017-06-04 09:00   14  CW    QP3GES        13M  599 26  -  1\r\n\
                    2017-06-04 09:01   21  CW    QC1UUB        599 100110  599 100121  -  1\r\n";
        let v = Jarl::default().decode(text.as_bytes()).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].rcvd.qxsl("call"), Some("QP3GES"));
        assert_eq!(v[0].sent.qxsl("rstq"), None);
        assert_eq!(v[1].rcvd.qxsl("code"), Some("100121"));
    }
}
