//! ADIF XML (ADX), QxSL's `adxs`.
//!
//! `<ADX><HEADER/><RECORDS><RECORD><CALL>...</CALL>...</RECORD></RECORDS></ADX>`;
//! every child of a record becomes an `adif.org` field named by its local name.

use crate::formats::xml::{escape, parse};
use crate::model::{Item, QName, ADIF};
use crate::{Error, Format, Result};

pub struct Adxs;

impl Format for Adxs {
    fn kind(&self) -> &'static str {
        "adxs"
    }

    fn name(&self) -> &'static str {
        "ADIF (ADX)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["adx"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let root = parse(bytes)?;
        if root.local() != "ADX" {
            return Err(Error::new("not an ADX document"));
        }
        let mut items = Vec::new();
        for list in root.children.iter().filter(|e| e.local() == "RECORDS") {
            for rec in list.children.iter().filter(|e| e.local() == "RECORD") {
                let mut item = Item::default();
                for f in &rec.children {
                    item.attrs.set(QName::new(ADIF, f.local()), f.text.clone());
                }
                items.push(item);
            }
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = String::from("<?xml version=\"1.0\" ?>\n<ADX>\n<HEADER></HEADER>\n<RECORDS>");
        for item in items {
            out.push_str("\n<RECORD>");
            for (k, v) in item.attrs.iter().filter(|(k, _)| k.space == ADIF) {
                out.push_str(&format!("<{0}>{1}</{0}>\n", k.local, escape(v)));
            }
            out.push_str("</RECORD>");
        }
        out.push_str("\n</RECORDS>\n</ADX>\n");
        Ok(out.into_bytes())
    }
}
