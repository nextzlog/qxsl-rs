//! QXML, QxSL's own XML format.
//!
//! `<list xmlns:QxSL="qxsl.org"><item qxsl:time=".." ..><rcvd ../><sent ../></item></list>`:
//! attributes of `item`, `rcvd` and `sent` are kept with their namespaces, so any
//! attribute survives a round trip.

use std::collections::BTreeMap;

use crate::formats::xml::{escape, local, parse, Element};
use crate::model::{Item, Node, QName, ADIF, QXSL};
use crate::{Error, Format, Result};

pub struct Qxml;

impl Format for Qxml {
    fn kind(&self) -> &'static str {
        "qxml"
    }

    fn name(&self) -> &'static str {
        "QXML"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["xml", "qxml"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let root = parse(bytes)?;
        if root.local() != "list" {
            return Err(Error::new("not a QXML document"));
        }
        let mut spaces = BTreeMap::new();
        declare(&root, &mut spaces);
        let mut items = Vec::new();
        for el in root.children.iter().filter(|e| e.local() == "item") {
            let mut spaces = spaces.clone();
            declare(el, &mut spaces);
            let mut item = Item {
                attrs: node(el, &spaces)?,
                ..Item::default()
            };
            for sub in &el.children {
                let mut spaces = spaces.clone();
                declare(sub, &mut spaces);
                match sub.local() {
                    "rcvd" => item.rcvd = node(sub, &spaces)?,
                    "sent" => item.sent = node(sub, &spaces)?,
                    _ => {}
                }
            }
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut prefixes: BTreeMap<String, String> = BTreeMap::new();
        let nodes = items.iter().flat_map(|i| [&i.attrs, &i.rcvd, &i.sent]);
        for n in nodes {
            for (k, _) in n.iter() {
                if !k.space.is_empty() && !prefixes.contains_key(&k.space) {
                    let p = match k.space.as_str() {
                        QXSL => "qxsl".to_string(),
                        ADIF => "adif".to_string(),
                        _ => format!("ns{}", prefixes.len() + 1),
                    };
                    prefixes.insert(k.space.clone(), p);
                }
            }
        }
        let mut out = String::from("<?xml version=\"1.0\" ?>\n<list");
        for (uri, p) in &prefixes {
            out.push_str(&format!(" xmlns:{p}=\"{}\"", escape(uri)));
        }
        out.push('>');
        for item in items {
            out.push_str("\n<item");
            attrs(&mut out, &item.attrs, &prefixes);
            out.push_str(">\n");
            for (tag, n) in [("rcvd", &item.rcvd), ("sent", &item.sent)] {
                if !n.is_empty() {
                    out.push_str(&format!("<{tag}"));
                    attrs(&mut out, n, &prefixes);
                    out.push_str("/>\n");
                }
            }
            out.push_str("</item>");
        }
        out.push_str("\n</list>\n");
        Ok(out.into_bytes())
    }
}

fn declare(el: &Element, spaces: &mut BTreeMap<String, String>) {
    for (k, v) in &el.attrs {
        if let Some(p) = k.strip_prefix("xmlns:") {
            spaces.insert(p.to_string(), v.clone());
        }
    }
}

fn node(el: &Element, spaces: &BTreeMap<String, String>) -> Result<Node> {
    let mut n = Node::default();
    for (k, v) in &el.attrs {
        if k == "xmlns" || k.starts_with("xmlns:") {
            continue;
        }
        let space = match k.split_once(':') {
            Some((p, _)) => spaces
                .get(p)
                .cloned()
                .ok_or_else(|| Error::new(format!("undeclared prefix: {p}")))?,
            None => String::new(),
        };
        n.set(QName::new(space, local(k)), v.clone());
    }
    Ok(n)
}

fn attrs(out: &mut String, n: &Node, prefixes: &BTreeMap<String, String>) {
    for (k, v) in n.iter() {
        match prefixes.get(&k.space) {
            Some(p) => out.push_str(&format!(" {p}:{}=\"{}\"", k.local, escape(v))),
            None => out.push_str(&format!(" {}=\"{}\"", k.local, escape(v))),
        }
    }
}
