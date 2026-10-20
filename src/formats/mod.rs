//! One module per format, each a [`Format`](crate::Format).

pub mod adis;
pub mod adxs;
pub mod cbin;
pub mod cqww;
pub mod ctxt;
pub mod hbin;
pub mod jarl;
pub mod jswl;
pub mod qxml;
pub mod zall;
pub mod zdos;

mod xml;

use crate::model::{Item, QName};

/// A `qxsl.org` attribute of the item, or an empty string.
pub(crate) fn some<'a>(item: &'a Item, local: &str) -> &'a str {
    item.attrs.qxsl(local).unwrap_or("")
}

/// Set a `qxsl.org` attribute unless the text is empty (the decoders skip blanks).
pub(crate) fn set_text(node: &mut crate::Node, local: &str, text: &str) {
    if !text.is_empty() {
        node.set(QName::qxsl(local), text);
    }
}

/// Set `rstq`, which must be an integer as in QxSL's `RSTQFactory`.
pub(crate) fn set_rstq(node: &mut crate::Node, text: &str) -> crate::Result<()> {
    if !text.is_empty() {
        let n: i32 = text
            .parse()
            .map_err(|_| crate::Error::new(format!("malformed RST: {text}")))?;
        node.set(QName::qxsl("rstq"), n.to_string());
    }
    Ok(())
}

/// Set `band` from a text that `BandFactory` parses.
pub(crate) fn set_band(node: &mut crate::Node, text: &str) -> crate::Result<()> {
    if !text.is_empty() {
        node.set(QName::qxsl("band"), crate::Band::parse(text)?.khz());
    }
    Ok(())
}

/// `%%name%%note` in the memo column of zLog's text formats.
pub(crate) fn set_name_note(node: &mut crate::Node, text: &str) {
    let found = text.get(2..).and_then(|t| t.find("%%")).map(|p| p + 2);
    let (name, note) = match found {
        Some(b) => (&text[2..b], &text[b + 2..]),
        None => ("", text),
    };
    node.set(QName::qxsl("name"), name.trim());
    node.set(QName::qxsl("note"), note.trim());
}

/// The band of an item in MHz as zLog's text formats write it (`14`, `1.9`, `1200`).
///
/// QxSL writes `toDecimalString(3)` cut to the column, which turns 10 GHz into
/// `1000`; bands from 10 GHz up are written as `10G`, `24G`, ... instead, which the
/// decoders read back.
pub(crate) fn zlog_band(item: &Item) -> String {
    match item
        .attrs
        .qxsl("band")
        .and_then(|b| crate::Band::parse(b).ok())
    {
        Some(b) if b.0 >= 10_000_000_000 => {
            let g = crate::Band(b.0 / 1000);
            format!("{}G", g.mhz())
        }
        Some(b) => b.mhz(),
        None => String::new(),
    }
}

/// A band in MHz as zLog's text formats write it, `G` meaning GHz.
pub(crate) fn zlog_band_text(text: &str) -> String {
    match text.strip_suffix('G') {
        Some(g) => format!("{g}GHz"),
        None if text.ends_with(|c: char| c.is_ascii_digit()) => format!("{text}MHz"),
        None => text.to_string(),
    }
}

/// A local time with an unknown year, read with QxSL's `getTimeDecoderOld`.
pub(crate) fn md_hm(
    month: &str,
    day: &str,
    hhmm: &str,
    year: i32,
    offset: i32,
) -> crate::Result<crate::Time> {
    let bad = || crate::Error::new(format!("malformed time: {month} {day} {hhmm}"));
    let m: u32 = month.trim().parse().map_err(|_| bad())?;
    let d: u32 = day.trim().parse().map_err(|_| bad())?;
    if hhmm.len() != 4 || !hhmm.is_ascii() {
        return Err(bad());
    }
    let h: u32 = hhmm[..2].parse().map_err(|_| bad())?;
    let mi: u32 = hhmm[2..].parse().map_err(|_| bad())?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || h > 23 || mi > 59 {
        return Err(bad());
    }
    Ok(crate::Time::new((year, m, d), (h, mi, 0), offset))
}

/// The item's time at the offset, if it has one.
pub(crate) fn local_time(item: &Item, offset: i32) -> Option<crate::Time> {
    item.attrs
        .qxsl("time")
        .and_then(|t| crate::Time::parse_iso(t).ok())
        .map(|t| t.at_offset(offset))
}
