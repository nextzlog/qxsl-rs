//! Turbo HAMLOG database (HDB), QxSL's `hbin`.
//!
//! A 32-byte header (`03 16 07 11`, record count as little-endian u32,
//! `21 02 D1 00`, zeros), sixteen 32-byte column descriptors (name, `C`, width at
//! byte 16), `0D`, then fixed-width Shift_JIS records each preceded by `20`, and
//! `1A` at the end. Dates are binary: century, year, month, day, hour, and minute
//! with bit 7 set for UTC (JST otherwise).

use crate::model::{Item, QName};
use crate::text::{decode_sjis, sjis_field};
use crate::value::JST;
use crate::{Band, Error, Format, Result, Time};

pub struct HBin;

/// Column names and widths in bytes, in file order.
pub const COLUMNS: &[(&str, usize)] = &[
    ("CALLS", 6),
    ("IGN", 14),
    ("DATE", 4),
    ("TIME", 2),
    ("CODE", 6),
    ("GL", 6),
    ("QSL", 3),
    ("FLAG", 2),
    ("HIS", 3),
    ("MY", 3),
    ("FREQ", 7),
    ("MODE", 4),
    ("NAME", 12),
    ("QTH", 28),
    ("RMK1", 54),
    ("RMK2", 54),
];

const HEAD: usize = 32 + 32 * 16;

impl Format for HBin {
    fn kind(&self) -> &'static str {
        "hbin"
    }

    fn name(&self) -> &'static str {
        "Turbo HAMLOG (HDB)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["hdb"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let bad = || Error::new("malformed HAMLOG DB sequence");
        let mut widths = Vec::new();
        for (i, (name, _)) in COLUMNS.iter().enumerate() {
            let d = bytes.get(32 + i * 32..64 + i * 32).ok_or_else(bad)?;
            if text(&d[..16]) != *name {
                return Err(bad());
            }
            widths.push(d[16] as usize);
        }
        let size: usize = widths.iter().sum();
        let mut pos = HEAD + 2;
        let mut items = Vec::new();
        while pos + size <= bytes.len() {
            let mut cols = Vec::new();
            let mut p = pos;
            for w in &widths {
                cols.push(&bytes[p..p + w]);
                p += w;
            }
            items.push(record(&cols)?);
            pos += size + 1;
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = vec![0x03, 0x16, 0x07, 0x11];
        out.extend((items.len() as u32).to_le_bytes());
        out.extend([0x21, 0x02, 0xD1, 0x00]);
        out.extend([0; 20]);
        for (name, width) in COLUMNS {
            let mut d = name.as_bytes().to_vec();
            d.resize(11, 0);
            d.push(0x43);
            d.extend([0; 4]);
            d.push(*width as u8);
            d.extend([0; 15]);
            out.extend(d);
        }
        out.push(0x0D);
        for item in items {
            out.push(0x20);
            out.extend(sjis_field(item.rcvd.qxsl("call").unwrap_or(""), 20, b' '));
            out.extend(time(item));
            out.extend(sjis_field(item.rcvd.qxsl("code").unwrap_or(""), 6, b' '));
            out.extend([b' '; 6 + 3 + 2]);
            out.extend(sjis_field(item.sent.qxsl("rstq").unwrap_or(""), 3, b' '));
            out.extend(sjis_field(item.rcvd.qxsl("rstq").unwrap_or(""), 3, b' '));
            let freq = item
                .attrs
                .qxsl("band")
                .and_then(|b| Band::parse(b).ok())
                .map(|b| b.mhz())
                .unwrap_or_default();
            out.extend(sjis_field(&freq, 7, b' '));
            out.extend(sjis_field(item.attrs.qxsl("mode").unwrap_or(""), 4, b' '));
            out.extend(sjis_field(item.attrs.qxsl("name").unwrap_or(""), 12, b' '));
            out.extend([b' '; 28]);
            out.extend(sjis_field(item.attrs.qxsl("note").unwrap_or(""), 54, b' '));
            out.extend(sjis_field(item.attrs.qxsl("text").unwrap_or(""), 54, b' '));
        }
        out.push(0x1A);
        Ok(out)
    }
}

/// A column as text: up to the first NUL (unless it is the first byte), trimmed.
fn text(bytes: &[u8]) -> String {
    let s = decode_sjis(bytes);
    let s = match s.find('\0') {
        Some(p) if p > 0 => &s[..p],
        _ => &s[..],
    };
    s.trim().to_string()
}

fn time(item: &Item) -> [u8; 6] {
    let Some(t) = item
        .attrs
        .qxsl("time")
        .and_then(|t| Time::parse_iso(t).ok())
    else {
        return [0; 6];
    };
    let utc = t.offset == 0;
    let v = t.at_offset(if utc { 0 } else { JST });
    [
        (v.year / 100) as u8,
        (v.year % 100) as u8,
        v.month as u8,
        v.day as u8,
        v.hour as u8,
        v.minute as u8 | if utc { 0x80 } else { 0 },
    ]
}

fn record(c: &[&[u8]]) -> Result<Item> {
    let mut item = Item::default();
    let set = |n: &mut crate::Node, k: &str, b: &[u8]| {
        let t = text(b);
        if !t.is_empty() {
            n.set(QName::qxsl(k), t);
        }
    };
    let call = [c[0], c[1]].concat();
    set(&mut item.rcvd, "call", &call);
    let dt = [c[2], c[3]].concat();
    if dt.len() >= 6 {
        let offset = if dt[5] & 0x80 != 0 { 0 } else { JST };
        let year = dt[0] as i32 * 100 + dt[1] as i32;
        let t = Time::new(
            (year, dt[2] as u32, dt[3] as u32),
            (dt[4] as u32, (dt[5] & 0x7F) as u32, 0),
            offset,
        );
        item.attrs.set(QName::qxsl("time"), t.to_iso());
    }
    set(&mut item.rcvd, "code", c[4]);
    set(&mut item.sent, "rstq", c[8]);
    set(&mut item.rcvd, "rstq", c[9]);
    let freq = text(c[10]);
    if !freq.is_empty() {
        let band = Band::parse(&format!("{freq}MHz"))?;
        item.attrs.set(QName::qxsl("band"), band.khz());
    }
    set(&mut item.attrs, "mode", c[11]);
    set(&mut item.attrs, "name", c[12]);
    set(&mut item.attrs, "note", c[14]);
    set(&mut item.attrs, "text", c[15]);
    Ok(item)
}
