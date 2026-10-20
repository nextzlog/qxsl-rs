//! ADIF tagged text (ADI), QxSL's `adis` ("Sub ADIF").
//!
//! Fields are `<NAME:LENGTH[:TYPE]>VALUE`; a header ends with `<EOH>` and every record
//! with `<EOR>` (any case). Names are kept upper-case in the `adif.org` namespace.
//!
//! ADI is ASCII: on writing, every non-ASCII character (such as Japanese text in a
//! memo or `COMMENT`) is replaced by `?`, and on reading bytes above 0x7F become
//! U+FFFD. ADX (`adxs`) is written as UTF-8 and keeps such characters.

use crate::model::{Item, QName, ADIF};
use crate::text::{decode_ascii, encode_ascii, NEWLINE};
use crate::{Error, Format, Result, Time};

pub struct Adis;

impl Format for Adis {
    fn kind(&self) -> &'static str {
        "adis"
    }

    fn name(&self) -> &'static str {
        "ADIF (ADI)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["adi"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let text = decode_ascii(bytes);
        let upper = text.to_ascii_uppercase();
        let mut pos = 0;
        let mut valid = false;
        if !text.starts_with('<') {
            match upper.find("<EOH>") {
                Some(p) => {
                    pos = p + 5;
                    valid = true;
                }
                None => pos = text.len(),
            }
        }
        let mut items = Vec::new();
        while let Some(p) = upper[pos..].find("<EOR>") {
            let record = &text[pos..pos + p];
            items.push(record_item(record)?);
            pos += p + 5;
            valid = true;
        }
        if !valid {
            return Err(Error::new("no header/records detected"));
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let now = Time::now(0);
        let stamp = format!(
            "{:04}{:02}{:02} {:02}{:02}{:02}",
            now.year, now.month, now.day, now.hour, now.minute, now.second
        );
        let mut out = format!(
            "ADIF\n<adif_ver:5>3.0.0\n<programid:4>qxsl\n<created_timestamp:{}>{stamp}<EOH>{NEWLINE}",
            stamp.len()
        );
        for item in items {
            for (k, v) in item.attrs.iter().filter(|(k, _)| k.space == ADIF) {
                out.push_str(&format!("<{}:{}>{v}", k.local, v.chars().count()));
            }
            out.push_str("<EOR>");
            out.push_str(NEWLINE);
        }
        Ok(encode_ascii(&out))
    }
}

/// The fields of one record, matched with `<(\w+?):(\d+?)(:\w)?>`.
fn record_item(record: &str) -> Result<Item> {
    let mut item = Item::default();
    let chars: Vec<char> = record.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '<' {
            i += 1;
            continue;
        }
        match tag(&chars, i + 1) {
            Some((name, len, end)) => {
                let value: String = chars[end..(end + len).min(chars.len())].iter().collect();
                item.attrs.set(QName::new(ADIF, name.to_uppercase()), value);
                i = end;
            }
            None => i += 1,
        }
    }
    Ok(item)
}

/// `NAME:LEN[:T]>` from position `i`: the name, the length and the index after `>`.
fn tag(c: &[char], mut i: usize) -> Option<(String, usize, usize)> {
    let word = |ch: char| ch.is_alphanumeric() || ch == '_';
    let start = i;
    while i < c.len() && word(c[i]) {
        i += 1;
    }
    if i == start || c.get(i) != Some(&':') {
        return None;
    }
    let name: String = c[start..i].iter().collect();
    i += 1;
    let ds = i;
    while i < c.len() && c[i].is_ascii_digit() {
        i += 1;
    }
    if i == ds {
        return None;
    }
    let len: usize = c[ds..i].iter().collect::<String>().parse().ok()?;
    if c.get(i) == Some(&':') {
        if !c.get(i + 1).is_some_and(|&ch| word(ch)) || c.get(i + 2) != Some(&'>') {
            return None;
        }
        i += 2;
    }
    if c.get(i) != Some(&'>') {
        return None;
    }
    Some((name, len, i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_header_and_types() {
        let src = b"ADIF export\n<EOH>\n<CALL:6>JA1ZLO<freq:6:N>14.025\n<EOR>\n";
        let items = Adis.decode(src).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].attrs.adif("CALL"), Some("JA1ZLO"));
        assert_eq!(items[0].attrs.adif("FREQ"), Some("14.025"));
        assert!(Adis.decode(b"nothing here").is_err());
    }
}
