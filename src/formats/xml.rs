//! A small element tree read with quick-xml, enough for ADX and QXML.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::{Error, Result};

#[derive(Debug, Default)]
pub struct Element {
    /// The name as written, with its prefix.
    pub name: String,
    /// Attributes as written (`prefix:local`, value).
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    pub fn local(&self) -> &str {
        local(&self.name)
    }
}

pub fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn err(e: impl std::fmt::Display) -> Error {
    Error::new(format!("malformed XML: {e}"))
}

fn open(e: &quick_xml::events::BytesStart<'_>) -> Result<Element> {
    let mut el = Element {
        name: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
        ..Element::default()
    };
    for a in e.attributes() {
        let a = a.map_err(err)?;
        let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let value = a
            .normalized_value(quick_xml::XmlVersion::default())
            .map_err(err)?
            .into_owned();
        el.attrs.push((key, value));
    }
    Ok(el)
}

fn entity(name: &str) -> Result<char> {
    let code = match name {
        "amp" => return Ok('&'),
        "lt" => return Ok('<'),
        "gt" => return Ok('>'),
        "quot" => return Ok('"'),
        "apos" => return Ok('\''),
        n if n.starts_with("#x") => u32::from_str_radix(&n[2..], 16).ok(),
        n if n.starts_with('#') => n[1..].parse().ok(),
        _ => None,
    };
    code.and_then(char::from_u32)
        .ok_or_else(|| err(format!("unknown entity &{name};")))
}

/// The root element of a UTF-8 document.
pub fn parse(bytes: &[u8]) -> Result<Element> {
    let text = std::str::from_utf8(bytes).map_err(err)?;
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<Element> = vec![Element::default()];
    loop {
        match reader.read_event().map_err(err)? {
            Event::Start(e) => stack.push(open(&e)?),
            Event::Empty(e) => {
                let el = open(&e)?;
                stack.last_mut().expect("root").children.push(el);
            }
            Event::End(_) => {
                let el = stack.pop().expect("open element");
                match stack.last_mut() {
                    Some(parent) => parent.children.push(el),
                    None => return Err(err("unbalanced end tag")),
                }
            }
            Event::Text(t) => {
                let s = t.xml10_content().map_err(err)?;
                stack.last_mut().expect("root").text.push_str(&s);
            }
            Event::CData(t) => {
                let s = t.decode().map_err(err)?;
                stack.last_mut().expect("root").text.push_str(&s);
            }
            Event::GeneralRef(r) => {
                let name = r.decode().map_err(err)?;
                let c = entity(&name)?;
                stack.last_mut().expect("root").text.push(c);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err(err("unclosed element"));
    }
    let doc = stack.pop().expect("document");
    doc.children
        .into_iter()
        .next()
        .ok_or_else(|| err("no root element"))
}

/// Text escaped for element content and attribute values.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}
