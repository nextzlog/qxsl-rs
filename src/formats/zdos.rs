//! zLog DOS text, QxSL's `zdos`.
//!
//! ```text
//!  MM  dd HHmm CCCCCCCCCC SSSSSSSSSSSS RRRRRRRRRRRR ****** BBBBB EEEE *** NNNN
//!   6   4 0915 QV1DOK     599100110    599120103              14 CW   1
//! ```
//!
//! Times are local and have no year; report and number share one column each.

use crate::formats::{
    local_time, md_hm, set_band, set_name_note, set_text, some, zlog_band, zlog_band_text,
};
use crate::model::{Item, QName};
use crate::text::{
    columns, current_year, decode_sjis, encode_sjis, lines, pad_tail, right, NEWLINE,
};
use crate::value::JST;
use crate::{Error, Format, Result};

pub struct ZDos {
    /// UTC offset of the times in the file, in minutes (QxSL uses the system zone).
    pub offset: i32,
    /// Year of the contacts; `None` means the current year, as in qxsl.
    pub year: Option<i32>,
}

impl Default for ZDos {
    fn default() -> Self {
        ZDos {
            offset: JST,
            year: None,
        }
    }
}

impl Format for ZDos {
    fn kind(&self) -> &'static str {
        "zdos"
    }

    fn name(&self) -> &'static str {
        "zLog (DOS)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txt"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let text = decode_sjis(bytes);
        let year = self.year.unwrap_or_else(current_year);
        let mut items = Vec::new();
        let mut head = true;
        for line in lines(&text) {
            if head && (line.trim().is_empty() || line.starts_with("mon")) {
                continue;
            }
            head = false;
            if line.trim().is_empty() {
                continue;
            }
            let v = columns(line, &[0, 13, 24, 37, 50, 57, 63, 68, 72, 157])?;
            let mut item = Item::default();
            if !v[0].is_empty() {
                let p: Vec<&str> = v[0].split_whitespace().collect();
                if p.len() != 3 {
                    return Err(Error::new(format!("malformed time: {}", v[0])));
                }
                let t = md_hm(p[0], p[1], p[2], year, self.offset)?;
                item.attrs.set(QName::qxsl("time"), t.to_iso());
            }
            set_text(&mut item.rcvd, "call", &v[1]);
            set_text(&mut item.sent, "code", &v[2]);
            set_text(&mut item.rcvd, "code", &v[3]);
            set_text(&mut item.attrs, "mul1", &v[4]);
            set_band(&mut item.attrs, &zlog_band_text(&v[5]))?;
            set_text(&mut item.attrs, "mode", &v[6]);
            if !v[8].is_empty() {
                set_name_note(&mut item.attrs, &v[8]);
            }
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = format!(
            "mon day time  callsign      sent         rcvd      multi   MHz mode pts memo{NEWLINE}"
        );
        for item in items {
            let t = local_time(item, self.offset)
                .map(|t| format!("{:>2}  {:>2} {:02}{:02}", t.month, t.day, t.hour, t.minute))
                .unwrap_or_else(|| " ".repeat(11));
            let cols = [
                String::new(),
                t,
                pad_tail(item.rcvd.qxsl("call").unwrap_or(""), 10),
                pad_tail(item.sent.qxsl("code").unwrap_or(""), 12),
                pad_tail(item.rcvd.qxsl("code").unwrap_or(""), 12),
                pad_tail(some(item, "mul1"), 6),
                right(&zlog_band(item), 5),
                pad_tail(some(item, "mode"), 4),
                "1  ".to_string(),
                format!("%%{}%%", some(item, "name")),
            ];
            for c in cols {
                out.push_str(&c);
                out.push(' ');
            }
            out.push_str(some(item, "note"));
            out.push_str(NEWLINE);
        }
        Ok(encode_sjis(&out))
    }
}
