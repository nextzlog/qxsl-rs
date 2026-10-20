//! Items and their namespaced attributes.
//!
//! As in QxSL, an attribute is named by a namespace URI and a local name. Values are
//! the strings QxSL's field factories produce: in the `qxsl.org` namespace `time` is an
//! ISO 8601 zoned date-time, `band` a frequency in kHz, `rstq` an integer, and the
//! others (`call`, `mode`, `code`, `name`, `note`, `mul1`, `mul2`, `watt`, `text`,
//! `sign`) plain text. ADIF fields live in `adif.org` with upper-case names.
//!
//! This crate adds three `qxsl.org` attributes of the item that QxSL lacks, for the
//! formats that carry them (JARL E-Log, Cabrillo): `points`, the points of the contact
//! as an integer; `newmult`, `true` or `false` as `mul1` is a new multiplier or not
//! (without it a format writes `mul1` as before); and `invalid`, `true` for a contact
//! that is not to be counted (absent otherwise). The memo of a contact is `note`.
//!
//! Unlike QxSL, which keeps one `call` on the item, `call` is split between the two
//! sub-nodes: the received node has the other station's callsign and the sent node the
//! own station's.

/// The namespace of QxSL's own attributes.
pub const QXSL: &str = "qxsl.org";

/// The namespace of ADIF fields.
pub const ADIF: &str = "adif.org";

/// A qualified attribute name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QName {
    pub space: String,
    pub local: String,
}

impl QName {
    pub fn new(space: impl Into<String>, local: impl Into<String>) -> Self {
        QName {
            space: space.into(),
            local: local.into(),
        }
    }

    /// A name in the `qxsl.org` namespace.
    pub fn qxsl(local: &str) -> Self {
        QName::new(QXSL, local)
    }

    /// A name in the `adif.org` namespace.
    pub fn adif(local: &str) -> Self {
        QName::new(ADIF, local)
    }

    fn is(&self, space: &str, local: &str) -> bool {
        self.space == space && self.local == local
    }
}

/// Attributes in the order they were set; setting an existing name replaces its value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Node {
    attrs: Vec<(QName, String)>,
}

impl Node {
    pub fn get_in(&self, space: &str, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k.is(space, local))
            .map(|(_, v)| v.as_str())
    }

    pub fn get(&self, key: &QName) -> Option<&str> {
        self.get_in(&key.space, &key.local)
    }

    /// A `qxsl.org` attribute.
    pub fn qxsl(&self, local: &str) -> Option<&str> {
        self.get_in(QXSL, local)
    }

    /// An `adif.org` attribute.
    pub fn adif(&self, local: &str) -> Option<&str> {
        self.get_in(ADIF, local)
    }

    pub fn set(&mut self, key: QName, value: impl Into<String>) {
        let value = value.into();
        match self.attrs.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.attrs.push((key, value)),
        }
    }

    /// Set the attribute, or remove it when `value` is `None` (QxSL's `setf` with null).
    pub fn put(&mut self, key: QName, value: Option<impl Into<String>>) {
        match value {
            Some(v) => self.set(key, v),
            None => self.remove(&key),
        }
    }

    pub fn remove(&mut self, key: &QName) {
        self.attrs.retain(|(k, _)| k != key);
    }

    pub fn contains(&self, key: &QName) -> bool {
        self.get(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&QName, &str)> {
        self.attrs.iter().map(|(k, v)| (k, v.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.attrs.len()
    }
}

/// One contact: its own attributes and the received and sent sub-nodes (the other
/// station's `call` in `rcvd`, the own station's in `sent`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Item {
    pub attrs: Node,
    pub rcvd: Node,
    pub sent: Node,
}
