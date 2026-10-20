//! JARL SWL log R2.1, QxSL's `jswl`.
//!
//! As the JARL log, but the sent column holds only the exchange heard from the
//! first station (no report); [`conv::normalize`](crate::conv::normalize) supplies
//! 59 or 599 as its report.

use crate::formats::jarl::{date_time, empty, records, set_mul1, time, HEADER};
use crate::formats::{set_band, set_rstq, set_text, some, zlog_band, zlog_band_text};
use crate::model::{Item, QName};
use crate::text::{decode_sjis, encode_sjis, pad_tail, right, NEWLINE};
use crate::value::JST;
use crate::{Format, Result};

pub struct Jswl {
    /// UTC offset of the times in the file, in minutes (JST in JARL's rules).
    pub offset: i32,
}

impl Default for Jswl {
    fn default() -> Self {
        Jswl { offset: JST }
    }
}

impl Format for Jswl {
    fn kind(&self) -> &'static str {
        "jswl"
    }

    fn name(&self) -> &'static str {
        "JARL SWL (R2.1)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txt"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let text = decode_sjis(bytes);
        let mut items = Vec::new();
        for v in records(&text) {
            let mut item = Item::default();
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
            if let Some(c) = v.get(5) {
                set_text(&mut item.sent, "code", c);
            }
            if let Some(r) = v.get(6) {
                set_rstq(&mut item.rcvd, r)?;
            }
            if let Some(c) = v.get(7) {
                set_text(&mut item.rcvd, "code", c);
            }
            if let Some(m) = v.get(8) {
                set_mul1(&mut item, m);
            }
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = format!("{HEADER}{NEWLINE}");
        for item in items {
            let cols = [
                date_time(item, self.offset),
                right(&zlog_band(item), 5),
                pad_tail(some(item, "mode"), 5),
                pad_tail(item.rcvd.qxsl("call").unwrap_or(""), 13),
                pad_tail(item.sent.qxsl("code").unwrap_or(""), 11),
                pad_tail(item.rcvd.qxsl("rstq").unwrap_or(""), 3),
                pad_tail(item.rcvd.qxsl("code").unwrap_or(""), 7),
                empty(pad_tail(some(item, "mul1"), 6)),
            ];
            for c in cols {
                out.push_str(&c);
                out.push(' ');
            }
            out.push_str("  1");
            out.push_str(NEWLINE);
        }
        Ok(encode_sjis(&out))
    }
}
