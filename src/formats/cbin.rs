//! CTESTWIN binary log (LG8), QxSL's `cbin`.
//!
//! Little-endian. A 16-byte header (QSO count as u16, `FF FF 00 00 08 00`,
//! `CQsoData`), then one record per contact separated by `01 80`:
//!
//! | bytes | field                                               |
//! |-------|-----------------------------------------------------|
//! | 20    | callsign (Shift_JIS, NUL-terminated)                |
//! | 30    | sent report and number                              |
//! | 30    | received report and number                          |
//! | 1     | mode index ([`MODES`])                              |
//! | 1     | (zero)                                              |
//! | 1     | band index ([`BANDS`])                              |
//! | 5     | (`00 0A 00 00 80` when written)                     |
//! | 8     | time, seconds since 1970-01-01 UTC (i64)            |
//! | 20    | operator                                            |
//! | 2     | (zero)                                              |
//! | 50    | note                                                |
//!
//! After the records QxSL writes the settings block CTESTWIN expects and up to 30
//! operator names.

use crate::model::{Item, QName};
use crate::text::{sjis_field, sjis_until_nul};
use crate::{Band, Error, Format, Result, Time};

pub struct CBin;

/// Bands by index, in kHz.
pub const BANDS: &[u64] = &[
    1900, 3500, 7000, 10000, 14000, 18000, 21000, 24000, 28000, 50000, 144000, 430000, 1200000,
    2400000, 5600000, 10000000, 24000000, 47000000, 75000000, 77000000, 135000000, 248000000, 136,
];

/// Modes by index.
pub const MODES: &[&str] = &[
    "CW", "RTTY", "SSB", "FM", "AM", "ATV", "SSTV", "PSK", "GMSK", "MFSK", "QPSK", "FSK", "D-STAR",
    "C4FM", "JT65", "JT9", "ISCAT", "FT8", "JT4", "QRA64", "MSK144", "WSPR", "JTMS", "FT4",
];

const MAGIC: &[u8] = b"CQsoData";
const RECORD: usize = 168;

impl Format for CBin {
    fn kind(&self) -> &'static str {
        "cbin"
    }

    fn name(&self) -> &'static str {
        "CTESTWIN (LG8)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["lg8"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        if bytes.len() < 16 || &bytes[8..16] != MAGIC {
            return Err(Error::new("malformed CTESTWIN sequence"));
        }
        let count = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        let mut items = Vec::with_capacity(count);
        let mut pos = 16;
        for _ in 0..count {
            let r = bytes
                .get(pos..pos + RECORD)
                .ok_or_else(|| Error::new("truncated CTESTWIN record"))?;
            items.push(record(r));
            pos += RECORD + 2;
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        out.extend((items.len() as u16).to_le_bytes());
        out.extend([0xFF, 0xFF, 0x00, 0x00, 0x08, 0x00]);
        out.extend(MAGIC);
        let mut names: Vec<&str> = Vec::new();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.extend([0x01, 0x80]);
            }
            out.extend(field(item.rcvd.qxsl("call"), 20));
            out.extend(field(item.sent.qxsl("code"), 30));
            out.extend(field(item.rcvd.qxsl("code"), 30));
            out.push(mode(item));
            out.push(0);
            out.push(band(item));
            out.extend([0x00, 0x0A, 0x00, 0x00, 0x80]);
            let t = item
                .attrs
                .qxsl("time")
                .and_then(|t| Time::parse_iso(t).ok())
                .map_or(0, |t| t.unix());
            out.extend(t.to_le_bytes());
            out.extend(field(item.attrs.qxsl("name"), 20));
            out.extend([0, 0]);
            out.extend(field(item.attrs.qxsl("note"), 50));
            let name = item.attrs.qxsl("name").unwrap_or("");
            if !names.contains(&name) {
                names.push(name);
            }
        }
        let last = items.last().cloned().unwrap_or_default();
        out.push(mode(&last));
        out.push(0);
        out.extend([0; 4]);
        out.push(band(&last));
        out.extend([0, 0, 0]);
        out.extend([0x00, 0x00]);
        for _ in 0..46 {
            out.extend([0x01, 0x00]);
        }
        for name in names.iter().take(30) {
            out.extend(sjis_field(name, 19, 0));
            out.push(0);
        }
        out.resize(out.len() + 30usize.saturating_sub(names.len()) * 20, 0);
        Ok(out)
    }
}

/// A text field cut to `size - 1` bytes and padded with NULs.
fn field(text: Option<&str>, size: usize) -> Vec<u8> {
    let mut b = sjis_field(text.unwrap_or(""), size - 1, 0);
    b.push(0);
    b
}

fn mode(item: &Item) -> u8 {
    item.attrs.qxsl("mode").map_or(0, |m| {
        MODES.iter().position(|x| *x == m).map_or(0xFF, |i| i as u8)
    })
}

fn band(item: &Item) -> u8 {
    match item.attrs.qxsl("band").and_then(|b| Band::parse(b).ok()) {
        None => 0,
        Some(b) => BANDS
            .iter()
            .position(|k| Band::from_khz(*k) == b)
            .map_or(0xFF, |i| i as u8),
    }
}

fn record(r: &[u8]) -> Item {
    let mut item = Item::default();
    item.rcvd
        .set(QName::qxsl("call"), sjis_until_nul(&r[0..20]));
    item.sent
        .set(QName::qxsl("code"), sjis_until_nul(&r[20..50]));
    item.rcvd
        .set(QName::qxsl("code"), sjis_until_nul(&r[50..80]));
    if let Some(m) = MODES.get(r[80] as usize) {
        item.attrs.set(QName::qxsl("mode"), *m);
    }
    if let Some(b) = BANDS.get(r[82] as usize) {
        item.attrs
            .set(QName::qxsl("band"), Band::from_khz(*b).khz());
    }
    let secs = i64::from_le_bytes(r[88..96].try_into().expect("8 bytes"));
    item.attrs
        .set(QName::qxsl("time"), Time::from_unix(secs, 0).to_iso());
    item.attrs
        .set(QName::qxsl("name"), sjis_until_nul(&r[96..116]));
    item.attrs
        .set(QName::qxsl("note"), sjis_until_nul(&r[118..168]));
    item
}
