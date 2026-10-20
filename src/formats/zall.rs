//! zLog ALL text, QxSL's `zall`.
//!
//! ```text
//! yyyy/MM/dd HH:mm CCCCCCCCCCCC sss SSSSSSS rrr RRRRRRR ***** ***** BBBB EEEE ** NNNN
//! 2017/06/04 09:15 QV1DOK       599 100110  599 120103                14 CW   1
//! ```
//!
//! Times are local; the band is in MHz; the memo column holds `%%operator%%note`.
//! A band longer than its four columns (`10.4G`) takes the separator after it as
//! zLog's five-character band column does (`10.4GSSB`).

use crate::formats::{
    local_time, set_band, set_name_note, set_rstq, set_text, some, zlog_band, zlog_band_text,
};
use crate::model::{Item, QName};
use crate::text::{columns, decode_sjis, encode_sjis, lines, pad_tail, right, NEWLINE};
use crate::value::JST;
use crate::{Error, Format, Result, Time};

pub struct ZAll {
    /// UTC offset of the times in the file, in minutes (QxSL uses the system zone).
    pub offset: i32,
}

impl Default for ZAll {
    fn default() -> Self {
        ZAll { offset: JST }
    }
}

impl Format for ZAll {
    fn kind(&self) -> &'static str {
        "zall"
    }

    fn name(&self) -> &'static str {
        "zLog (ALL)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["all"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let text = decode_sjis(bytes);
        let mut items = Vec::new();
        let mut head = true;
        for line in lines(&text) {
            if head
                && (line.trim().is_empty() || line.starts_with("zLog") || line.starts_with("Date"))
            {
                continue;
            }
            head = false;
            if line.trim().is_empty() {
                continue;
            }
            let chars: Vec<char> = line.chars().collect();
            // A band of five characters fills the separator after its column.
            let wide = chars.len() > 70 && chars[70] != ' ';
            let line: String = if wide {
                let mut c = chars.clone();
                c[66..71].fill(' ');
                c.into_iter().collect()
            } else {
                line.to_string()
            };
            let mut v = columns(&line, &[0, 17, 30, 34, 42, 46, 54, 60, 66, 71, 76, 79, 164])?;
            if wide {
                v[8] = chars[66..71].iter().collect::<String>().trim().to_string();
            }
            let mut item = Item::default();
            if !v[0].is_empty() {
                item.attrs
                    .set(QName::qxsl("time"), time(&v[0], self.offset)?.to_iso());
            }
            set_text(&mut item.rcvd, "call", &v[1]);
            set_rstq(&mut item.sent, &v[2])?;
            set_text(&mut item.sent, "code", &v[3]);
            set_rstq(&mut item.rcvd, &v[4])?;
            set_text(&mut item.rcvd, "code", &v[5]);
            set_text(&mut item.attrs, "mul1", &v[6]);
            set_text(&mut item.attrs, "mul2", &v[7]);
            set_band(&mut item.attrs, &zlog_band_text(&v[8]))?;
            set_text(&mut item.attrs, "mode", &v[9]);
            if !v[11].is_empty() {
                set_name_note(&mut item.attrs, &v[11]);
            }
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = format!("zLog for Windows{NEWLINE}");
        for item in items {
            let t = local_time(item, self.offset)
                .map(|t| {
                    format!(
                        "{:04}/{:02}/{:02} {:02}:{:02}",
                        t.year, t.month, t.day, t.hour, t.minute
                    )
                })
                .unwrap_or_else(|| " ".repeat(16));
            let cols = [
                t,
                pad_tail(item.rcvd.qxsl("call").unwrap_or(""), 12),
                pad_tail(item.sent.qxsl("rstq").unwrap_or(""), 3),
                pad_tail(item.sent.qxsl("code").unwrap_or(""), 7),
                pad_tail(item.rcvd.qxsl("rstq").unwrap_or(""), 3),
                pad_tail(item.rcvd.qxsl("code").unwrap_or(""), 7),
                pad_tail(some(item, "mul1"), 5),
                pad_tail(some(item, "mul2"), 5),
                band(item),
                pad_tail(some(item, "mode"), 4),
                "1 ".to_string(),
                format!("%%{}%%", some(item, "name")),
            ];
            for (i, c) in cols.iter().enumerate() {
                out.push_str(c);
                if i != BAND || c.chars().count() <= 4 {
                    out.push(' ');
                }
            }
            out.push_str(some(item, "note"));
            out.push_str(NEWLINE);
        }
        Ok(encode_sjis(&out))
    }
}

/// The index of the band among the written columns.
const BAND: usize = 8;

/// The band right-aligned in four columns, or in five (zLog's width) if longer.
fn band(item: &Item) -> String {
    let b = zlog_band(item);
    if b.chars().count() > 4 {
        pad_tail(&b, 5)
    } else {
        right(&b, 4)
    }
}

/// `uuuu/MM/dd HH:mm`.
fn time(text: &str, offset: i32) -> Result<Time> {
    let bad = || Error::new(format!("malformed time: {text}"));
    let (d, hm) = text.split_once(' ').ok_or_else(bad)?;
    Time::parse_iso(&format!("{}T{}", d.replace('/', "-"), hm.trim()))
        .or_else(|_| Time::parse_iso(&format!("{}T{}Z", d.replace('/', "-"), hm.trim())))
        .map(|t| Time { offset, ..t })
        .map_err(|_| bad())
}
