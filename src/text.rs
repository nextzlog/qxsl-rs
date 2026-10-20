//! Helpers for the text formats: character sets, lines and fixed columns.

use crate::{Error, Result};

/// Line separator written by the encoders (QxSL uses the platform's; zLog and
/// CTESTWIN are Windows programs).
pub const NEWLINE: &str = "\r\n";

pub fn decode_sjis(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned()
}

pub fn encode_sjis(text: &str) -> Vec<u8> {
    encoding_rs::SHIFT_JIS.encode(text).0.into_owned()
}

/// Text read as ASCII: bytes above 0x7F become U+FFFD, as Java's ASCII decoder does.
pub fn decode_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| if b < 0x80 { b as char } else { '\u{FFFD}' })
        .collect()
}

pub fn encode_ascii(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
        .collect()
}

/// Lines as `BufferedReader.readLine` splits them (`\n`, `\r\n` or `\r`).
pub fn lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        match rest.find(['\r', '\n']) {
            Some(p) => {
                out.push(&rest[..p]);
                let skip = if rest[p..].starts_with("\r\n") { 2 } else { 1 };
                rest = &rest[p + skip..];
            }
            None => {
                out.push(rest);
                rest = "";
            }
        }
    }
    out
}

/// `PrintDecoder.split`: the fields between the column positions, trimmed.
///
/// Every field but the last must be followed by a space. Positions count characters.
/// Columns beyond the end of the line read as blank.
pub fn columns(line: &str, cols: &[usize]) -> Result<Vec<String>> {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut out = Vec::with_capacity(cols.len() - 1);
    for i in 1..cols.len() {
        let head = cols[i - 1].min(n);
        let last = (cols[i] - 1).min(n);
        if i + 1 < cols.len() && last < n && chars[last] != ' ' {
            return Err(Error::new(format!("malformed record: {line}")));
        }
        let field: String = chars[head..last].iter().collect();
        out.push(field.trim().to_string());
    }
    Ok(out)
}

/// `Field.padTail`: cut to `size` characters, then pad with spaces on the right.
pub fn pad_tail(text: &str, size: usize) -> String {
    let t: String = text.chars().take(size).collect();
    let n = t.chars().count();
    t + &" ".repeat(size - n)
}

/// `Field.padHead`: cut to `size` characters, then pad with spaces on the left.
pub fn pad_head(text: &str, size: usize) -> String {
    let t: String = text.chars().take(size).collect();
    let n = t.chars().count();
    " ".repeat(size - n) + &t
}

/// `text` padded with spaces on the right to `size` characters, never cut (the
/// `FillRight` of zLog's `SaveToFileAsCabrillo`).
pub fn fill_tail(text: &str, size: usize) -> String {
    let n = text.chars().count();
    text.to_string() + &" ".repeat(size.saturating_sub(n))
}

/// `text` padded with spaces on the left to `size` characters, never cut (the
/// `FillLeft` of zLog's `SaveToFileAsCabrillo`).
pub fn fill_head(text: &str, size: usize) -> String {
    let n = text.chars().count();
    " ".repeat(size.saturating_sub(n)) + text
}

/// `String.format("%N.Ns", text)`: right-aligned and cut to `size` characters.
pub fn right(text: &str, size: usize) -> String {
    pad_head(text, size)
}

/// Shift_JIS bytes of `text`, cut at a character boundary to at most `size` bytes.
pub fn sjis_cut(text: &str, size: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for c in text.chars() {
        let b = encode_sjis(c.encode_utf8(&mut [0; 4]));
        if out.len() + b.len() > size {
            break;
        }
        out.extend(b);
    }
    out
}

/// A fixed-width Shift_JIS field: `text` cut to `size` bytes, padded with `fill`.
pub fn sjis_field(text: &str, size: usize, fill: u8) -> Vec<u8> {
    let mut b = sjis_cut(text, size);
    b.resize(size, fill);
    b
}

/// A Shift_JIS string that ends at the first NUL.
pub fn sjis_until_nul(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    decode_sjis(&bytes[..end])
}

/// The current year, which QxSL assumes for dates written without one.
pub fn current_year() -> i32 {
    crate::value::Time::now(crate::value::JST).year
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_columns() {
        let v = columns("ab   cd  e", &[0, 5, 9, 20]).unwrap();
        assert_eq!(v, ["ab", "cd", "e"]);
        assert!(columns("abcdefgh", &[0, 3, 9]).is_err());
        assert_eq!(lines("a\r\nb\nc\rd"), ["a", "b", "c", "d"]);
        assert_eq!(pad_tail("abc", 2), "ab");
        assert_eq!(pad_head("7", 3), "  7");
        assert_eq!(fill_tail("abc", 2), "abc");
        assert_eq!(fill_tail("a", 3), "a  ");
        assert_eq!(fill_head("abcdef", 5), "abcdef");
        assert_eq!(fill_head("7", 3), "  7");
    }
}
