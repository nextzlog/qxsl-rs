//! CTESTWIN text, QxSL's `ctxt`.
//!
//! ```text
//! **** MM/dd HHmm CCCCCCCCCCC BBBBBBB EEEE SSSSSSSSSSSS RRRRRRRRRRRR
//!    1  6/ 4 0915 QV1DOK        14MHz CW   599100110    599120103
//! ```
//!
//! Times are local and have no year; report and number share one column each.

use crate::formats::{local_time, md_hm, set_band, set_text, some};
use crate::model::{Item, QName};
use crate::text::{
    columns, current_year, decode_sjis, encode_sjis, lines, pad_head, pad_tail, NEWLINE,
};
use crate::value::JST;
use crate::{Band, Error, Format, Result};

pub struct CTxt {
    /// UTC offset of the times in the file, in minutes (QxSL uses the system zone).
    pub offset: i32,
    /// Year of the contacts; `None` means the current year, as in qxsl.
    pub year: Option<i32>,
}

impl Default for CTxt {
    fn default() -> Self {
        CTxt {
            offset: JST,
            year: None,
        }
    }
}

impl Format for CTxt {
    fn kind(&self) -> &'static str {
        "ctxt"
    }

    fn name(&self) -> &'static str {
        "CTESTWIN (text)"
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
            if head && (line.trim().is_empty() || line.starts_with("Worked")) {
                continue;
            }
            head = false;
            if line.trim().is_empty() {
                continue;
            }
            let v = columns(line, &[0, 5, 16, 28, 36, 41, 54, 67])?;
            v[0].parse::<i32>()
                .map_err(|_| Error::new(format!("malformed record: {line}")))?;
            let mut item = Item::default();
            if !v[1].is_empty() {
                let (md, hm) = v[1]
                    .rsplit_once(' ')
                    .ok_or_else(|| Error::new(format!("malformed time: {}", v[1])))?;
                let (m, d) = md
                    .split_once('/')
                    .ok_or_else(|| Error::new(format!("malformed time: {}", v[1])))?;
                let t = md_hm(m, d, hm, year, self.offset)?;
                item.attrs.set(QName::qxsl("time"), t.to_iso());
            }
            set_text(&mut item.rcvd, "call", &v[2]);
            set_band(&mut item.attrs, &v[3])?;
            set_text(&mut item.attrs, "mode", &v[4]);
            set_text(&mut item.sent, "code", &v[5]);
            set_text(&mut item.rcvd, "code", &v[6]);
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = format!("Worked {:>4} stations{NEWLINE}{NEWLINE}", items.len());
        for (n, item) in items.iter().enumerate() {
            let t = local_time(item, self.offset)
                .map(|t| format!("{:>2}/{:>2} {:02}{:02}", t.month, t.day, t.hour, t.minute))
                .unwrap_or_else(|| " ".repeat(10));
            let band = item
                .attrs
                .qxsl("band")
                .and_then(|b| Band::parse(b).ok())
                .map(|b| b.label())
                .unwrap_or_default();
            let cols = [
                format!("{:>4}", n + 1),
                t,
                pad_tail(item.rcvd.qxsl("call").unwrap_or(""), 11),
                pad_head(&band, 7),
                pad_tail(some(item, "mode"), 4),
                pad_tail(item.sent.qxsl("code").unwrap_or(""), 12),
                pad_tail(item.rcvd.qxsl("code").unwrap_or(""), 12),
            ];
            for c in cols {
                out.push_str(&c);
                out.push(' ');
            }
            out.push_str(NEWLINE);
        }
        Ok(encode_sjis(&out))
    }
}
