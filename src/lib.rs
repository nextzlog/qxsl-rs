//! Amateur-radio log file formats.
//!
//! Ported from QxSL (https://github.com/autodyne/qxsl, LGPL-3.0) by permission of its
//! author, who allows this port to be distributed under the MIT License.
//!
//! The model mirrors QxSL: a log is a list of [`Item`]s; an item holds namespaced
//! attributes of the contact and two sub-nodes with what was received and what was
//! sent; the other station's callsign is the received `call` and the own station's
//! the sent `call`. Every value is kept as the string QxSL would write for it (see
//! [`model`]), so attributes this crate does not know survive a round trip.
//!
//! Each format decodes bytes into items carrying its own attributes (ADIF fields in
//! the `adif.org` namespace, the others in `qxsl.org`) and encodes items back. The
//! conversions between those conventions, which QxSL keeps in `format.lisp`, are in
//! [`conv`]: [`conv::normalize`] brings a decoded item into the common QxSL form and
//! [`conv::transform`] prepares a common item for one format. [`Format::read`] and
//! [`Format::write`] combine the two steps.
//!
//! | type id | format                         | module                  |
//! |---------|--------------------------------|-------------------------|
//! | `adis`  | ADIF, tagged text (ADI)        | [`formats::adis`]       |
//! | `adxs`  | ADIF, XML (ADX)                | [`formats::adxs`]       |
//! | `cbin`  | CTESTWIN binary (LG8)          | [`formats::cbin`]       |
//! | `ctxt`  | CTESTWIN text                  | [`formats::ctxt`]       |
//! | `cqww`  | Cabrillo (CQWW)                | [`formats::cqww`]       |
//! | `hbin`  | Turbo HAMLOG (HDB)             | [`formats::hbin`]       |
//! | `jarl`  | JARL log R2.1                  | [`formats::jarl`]       |
//! | `jswl`  | JARL SWL log R2.1              | [`formats::jswl`]       |
//! | `qxml`  | QXML                           | [`formats::qxml`]       |
//! | `zall`  | zLog ALL text                  | [`formats::zall`]       |
//! | `zdos`  | zLog DOS text                  | [`formats::zdos`]       |
//!
//! Not ported: QxSL's ZLO/ZLOX binaries (`zbin`, `znew`), Java object serialisation
//! (`item`), the summary sheets (`gaas.sheet`), and the rule and scoring engine.

pub mod conv;
pub mod formats;
pub mod model;
pub mod text;
pub mod value;

pub use model::{Item, Node, QName, ADIF, QXSL};
pub use value::{Band, Time};

/// An error while decoding or encoding a log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Error(message.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// One log file format.
pub trait Format: Send + Sync {
    /// Type id as in QxSL (`adis`, `cqww`, ...); selects the conversions in [`conv`].
    fn kind(&self) -> &'static str;

    /// Name for file dialogs.
    fn name(&self) -> &'static str;

    /// File extensions without the dot.
    fn extensions(&self) -> &'static [&'static str];

    /// Items with this format's own attributes.
    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>>;

    /// File contents for items that carry this format's own attributes.
    ///
    /// Attributes the format has no place for are left out.
    fn encode(&self, items: &[Item]) -> Result<Vec<u8>>;

    /// Decode and bring every item into the common QxSL form.
    fn read(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        Ok(self
            .decode(bytes)?
            .iter()
            .map(|i| conv::normalize(i, self.kind()))
            .collect())
    }

    /// Convert items in the common QxSL form for this format and encode them.
    fn write(&self, items: &[Item]) -> Result<Vec<u8>> {
        let items: Vec<Item> = items
            .iter()
            .map(|i| conv::transform(i, self.kind()))
            .collect();
        self.encode(&items)
    }
}

/// Every format of this crate, with default settings.
pub fn formats() -> Vec<Box<dyn Format>> {
    use formats::*;
    vec![
        Box::new(qxml::Qxml),
        Box::new(adis::Adis),
        Box::new(adxs::Adxs),
        Box::new(cqww::Cqww),
        Box::new(jarl::Jarl::default()),
        Box::new(jswl::Jswl::default()),
        Box::new(zall::ZAll::default()),
        Box::new(zdos::ZDos::default()),
        Box::new(ctxt::CTxt::default()),
        Box::new(cbin::CBin),
        Box::new(hbin::HBin),
    ]
}
