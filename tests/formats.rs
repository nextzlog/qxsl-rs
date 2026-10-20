//! Decoding QxSL's sample logs and round trips through every format.

use qxsl::formats::{adis::Adis, adxs::Adxs, cbin::CBin, cqww::Cqww, ctxt::CTxt, hbin::HBin};
use qxsl::formats::{jarl::Jarl, jswl::Jswl, qxml::Qxml, zall::ZAll, zdos::ZDos};
use qxsl::{Format, Item, QName, Time};

fn sample(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// The fields compared between formats: instant (to the minute), the other station's call, band, mode, received report
/// and number.
fn core(item: &Item) -> (i64, String, String, String, String, String) {
    let a = |k: &str| item.attrs.qxsl(k).unwrap_or("").to_string();
    let t = Time::parse_iso(&a("time"))
        .map(|t| t.unix() / 60)
        .unwrap_or(0);
    (
        t,
        item.rcvd.qxsl("call").unwrap_or("").to_string(),
        a("band"),
        a("mode"),
        item.rcvd.qxsl("rstq").unwrap_or("").to_string(),
        item.rcvd.qxsl("code").unwrap_or("").to_string(),
    )
}

fn reference() -> Vec<Item> {
    Qxml.read(&sample("allja1.qxml")).unwrap()
}

#[test]
fn samples_decode_to_the_same_contacts() {
    let want: Vec<_> = reference().iter().map(core).collect();
    assert_eq!(want.len(), 1000);
    let formats: Vec<(&str, Box<dyn Format>)> = vec![
        ("allja1.zall", Box::new(ZAll::default())),
        ("allja1.adxs", Box::new(Adxs)),
        ("allja1.cbin", Box::new(CBin)),
        ("allja1.cqww", Box::new(Cqww)),
        ("allja1.jarl", Box::new(Jarl::default())),
        ("allja1.zall", Box::new(ZAll::default())),
        (
            "allja1.zdos",
            Box::new(ZDos {
                year: Some(2017),
                ..ZDos::default()
            }),
        ),
        (
            "allja1.ctxt",
            Box::new(CTxt {
                year: Some(2017),
                ..CTxt::default()
            }),
        ),
    ];
    for (file, f) in formats {
        let want: Vec<_> = reference()
            .iter()
            .map(|i| qxsl::conv::normalize(&qxsl::conv::transform(i, f.kind()), f.kind()))
            .map(|i| core(&i))
            .collect();
        let got: Vec<_> = f.read(&sample(file)).unwrap().iter().map(core).collect();
        // The zLog and CTESTWIN text samples hold only the first 776 contacts.
        assert!(got.len() == want.len() || got.len() == 776, "{file}");
        // CTESTWIN and zLog DOS text have no year; the sample spans two contests.
        let yearless = file.ends_with("ctxt") || file.ends_with("zdos");
        let strip = |c: &(i64, String, String, String, String, String)| {
            let mut c = c.clone();
            if yearless {
                let t = Time::from_unix(c.0 * 60, 540);
                c.0 = (t.month * 1_000_000 + t.day * 10_000 + t.hour * 100 + t.minute) as i64;
            }
            c
        };
        for (i, (g, w)) in got
            .iter()
            .map(strip)
            .zip(want.iter().map(strip))
            .enumerate()
        {
            assert_eq!(g, w, "{file} record {i}");
        }
    }
}

#[test]
fn online_sample_reads() {
    let items = Adxs.read(&sample("online.adxs")).unwrap();
    assert!(!items.is_empty());
    let first = &items[0];
    assert_eq!(first.attrs.qxsl("time"), Some("2021-02-20T01:52:55Z"));
    assert_eq!(first.attrs.qxsl("band"), Some("7000"));
    assert_eq!(first.attrs.qxsl("name"), Some("6"));
    assert_eq!(first.sent.qxsl("code"), Some("100105"));
}

/// Items in the common form with the fields that every format carries plus names and
/// notes; phone contacts included so that reports of two digits are exercised.
fn contacts() -> Vec<Item> {
    let mut items: Vec<Item> = reference().into_iter().take(50).collect();
    for (i, it) in items.iter_mut().enumerate() {
        if i % 3 == 0 {
            it.attrs.set(QName::qxsl("mode"), "SSB");
            it.rcvd.set(QName::qxsl("rstq"), "59");
            it.sent.set(QName::qxsl("rstq"), "59");
        }
        let t = Time::parse_iso(it.attrs.qxsl("time").unwrap()).unwrap();
        it.attrs.set(QName::qxsl("time"), t.at_offset(540).to_iso());
    }
    items
}

fn round_trip(f: &dyn Format, fields: &[&str], sent: bool) {
    let items = contacts();
    let bytes = f.write(&items).unwrap();
    let back = f.read(&bytes).unwrap();
    assert_eq!(back.len(), items.len(), "{}", f.kind());
    for (a, b) in items.iter().zip(&back) {
        assert_eq!(core(a), core(b), "{}", f.kind());
        for k in fields {
            assert_eq!(a.attrs.qxsl(k), b.attrs.qxsl(k), "{} {k}", f.kind());
        }
        if sent {
            let mut x: Vec<_> = a.sent.iter().collect();
            let mut y: Vec<_> = b.sent.iter().collect();
            x.sort();
            y.sort();
            assert_eq!(x, y, "{}", f.kind());
        }
    }
}

#[test]
fn round_trips() {
    round_trip(&Qxml, &["name", "note"], true);
    round_trip(&Adis, &["name"], true);
    round_trip(&Adxs, &["name"], true);
    round_trip(&Cqww, &[], true);
    round_trip(&Jarl::default(), &[], true);
    round_trip(&Jswl::default(), &[], false);
    round_trip(&ZAll::default(), &["name"], true);
    let year = Time::parse_iso(contacts()[0].attrs.qxsl("time").unwrap())
        .unwrap()
        .year;
    let zdos = ZDos {
        year: Some(year),
        ..ZDos::default()
    };
    round_trip(&zdos, &["name"], true);
    let ctxt = CTxt {
        year: Some(year),
        ..CTxt::default()
    };
    round_trip(&ctxt, &[], true);
    round_trip(&CBin, &["name"], true);
    round_trip(&HBin, &["name"], false);
}

#[test]
fn adif_is_lossless() {
    let src =
        "header\n<EOH>\n<CALL:6>JA1ZLO<QSO_DATE:8>20170604<TIME_ON:4>0915<BAND:3>20m<MODE:2>CW\
<RST_RCVD:3>599<SRX:2>13<APP_N1MM_EXCHANGE1:3>13M<COMMENT:5>hello<MY_GRIDSQUARE:4>PM95<EOR>\n";
    for f in [&Adis as &dyn Format, &Adxs] {
        let items = if f.kind() == "adis" {
            f.decode(src.as_bytes()).unwrap()
        } else {
            Adxs.decode(&Adxs.encode(&Adis.decode(src.as_bytes()).unwrap()).unwrap())
                .unwrap()
        };
        let again = f.decode(&f.encode(&items).unwrap()).unwrap();
        assert_eq!(items, again);
        assert_eq!(again[0].attrs.adif("MY_GRIDSQUARE"), Some("PM95"));
        // Through the common form and back the unknown fields survive too.
        let common = f.read(&f.encode(&items).unwrap()).unwrap();
        assert_eq!(common[0].rcvd.qxsl("code"), Some("13M"));
        let out = f.decode(&f.write(&common).unwrap()).unwrap();
        assert_eq!(out[0].attrs.adif("COMMENT"), Some("hello"));
        assert_eq!(out[0].attrs.adif("MY_GRIDSQUARE"), Some("PM95"));
        assert_eq!(out[0].attrs.adif("TIME_ON"), Some("091500"));
    }
}

#[test]
fn qxml_is_lossless() {
    let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<list xmlns:qxsl="qxsl.org" xmlns:x="example.org/x">
<item qxsl:time="2017-06-04T00:00:00Z" qxsl:call="QP3GES" x:rig="IC-7300 &amp; amp" plain="1">
<rcvd qxsl:code="26" qxsl:rstq="599" x:pwr="H"/>
<sent qxsl:code="100110" qxsl:rstq="599"/>
</item>
</list>"#;
    let items = Qxml.decode(src.as_bytes()).unwrap();
    assert_eq!(
        items[0].attrs.get(&QName::new("example.org/x", "rig")),
        Some("IC-7300 & amp")
    );
    assert_eq!(items[0].attrs.get(&QName::new("", "plain")), Some("1"));
    let again = Qxml.decode(&Qxml.encode(&items).unwrap()).unwrap();
    assert_eq!(items, again);
}

#[test]
fn exact_layouts() {
    let item = &contacts()[1];
    let line = |f: &dyn Format, n: usize| {
        let b = f.write(std::slice::from_ref(item)).unwrap();
        let t = encoding_free(&b);
        t.lines().nth(n).unwrap().to_string()
    };
    assert_eq!(
        line(&Cqww, 0),
        "QSO: 21000 CW 2017-06-04 0001 ************* 599 100110 QC1UUB        599 100121 "
    );
    assert_eq!(
        line(&ZAll::default(), 1),
        "2017/06/04 09:01 QC1UUB       599 100110  599 100121                21 CW   1  %%OP5%% "
    );
    assert_eq!(
        line(&Jarl::default(), 1),
        "2017-06-04 09:01    21 CW    QC1UUB        599 100110  599 100121  -        1"
    );
    assert_eq!(
        line(&ZDos::default(), 1),
        "  6   4 0901 QC1UUB     599100110    599100121              21 CW   1   %%OP5%% "
    );
    assert_eq!(
        line(&CTxt::default(), 2),
        "   1  6/ 4 0901 QC1UUB        21MHz CW   599100110    599100121    "
    );
}

fn encoding_free(b: &[u8]) -> String {
    String::from_utf8(b.to_vec()).unwrap()
}

/// Every format, with the settings the round trips use.
fn all_formats() -> Vec<Box<dyn Format>> {
    let year = Time::parse_iso(contacts()[0].attrs.qxsl("time").unwrap())
        .unwrap()
        .year;
    vec![
        Box::new(Qxml),
        Box::new(Adis),
        Box::new(Adxs),
        Box::new(Cqww),
        Box::new(Jarl::default()),
        Box::new(Jswl::default()),
        Box::new(ZAll::default()),
        Box::new(ZDos {
            year: Some(year),
            ..ZDos::default()
        }),
        Box::new(CTxt {
            year: Some(year),
            ..CTxt::default()
        }),
        Box::new(CBin),
        Box::new(HBin),
    ]
}

#[test]
fn peer_call_is_in_rcvd_for_every_format() {
    let items = contacts();
    for f in all_formats() {
        let back = f.read(&f.write(&items).unwrap()).unwrap();
        for (a, b) in items.iter().zip(&back) {
            assert!(a.rcvd.qxsl("call").is_some(), "{}", f.kind());
            assert_eq!(a.rcvd.qxsl("call"), b.rcvd.qxsl("call"), "{}", f.kind());
            assert_eq!(b.attrs.qxsl("call"), None, "{}", f.kind());
            assert_eq!(b.sent.qxsl("call"), None, "{}", f.kind());
        }
    }
}

#[test]
fn own_call_round_trips_where_the_format_has_a_field() {
    let mut items = contacts();
    for it in &mut items {
        it.sent.set(QName::qxsl("call"), "JA1ZLO");
    }
    for f in all_formats() {
        let back = f.read(&f.write(&items).unwrap()).unwrap();
        let carries = matches!(f.kind(), "qxml" | "adis" | "adxs" | "cqww");
        for b in &back {
            let want = if carries { Some("JA1ZLO") } else { None };
            assert_eq!(b.sent.qxsl("call"), want, "{}", f.kind());
            assert!(b.rcvd.qxsl("call").is_some(), "{}", f.kind());
        }
    }
    let adif = String::from_utf8(Adis.write(&items[..1]).unwrap()).unwrap();
    assert!(adif.contains("<STATION_CALLSIGN:6>JA1ZLO"), "{adif}");
}

#[test]
fn legacy_qxml_call_reads_into_rcvd_and_decodes_unchanged() {
    let raw = sample("allja1.qxml");
    let items = Qxml.decode(&raw).unwrap();
    assert_eq!(items[0].attrs.qxsl("call"), Some("QP3GES"));
    let common = Qxml.read(&raw).unwrap();
    assert_eq!(common[0].attrs.qxsl("call"), None);
    assert_eq!(common[0].rcvd.qxsl("call"), Some("QP3GES"));
    assert_eq!(common[0].sent.qxsl("call"), None);
}

#[test]
fn samples_reencode_byte_identical() {
    let formats: Vec<(&str, Box<dyn Format>)> = vec![
        ("allja1.cqww", Box::new(Cqww)),
        ("allja1.zall", Box::new(ZAll::default())),
        ("allja1.cbin", Box::new(CBin)),
    ];
    for (file, f) in formats {
        let raw = sample(file);
        let once = f.encode(&f.decode(&raw).unwrap()).unwrap();
        let twice = f.encode(&f.decode(&once).unwrap()).unwrap();
        assert_eq!(once, twice, "{file}");
    }
}

/// One contact in the common form, for the tests of single fields.
fn one(band: &str) -> Item {
    let mut it = Item::default();
    it.attrs
        .set(QName::qxsl("time"), "2017-06-04T09:15:00+09:00");
    it.attrs.set(QName::qxsl("band"), band);
    it.attrs.set(QName::qxsl("mode"), "CW");
    it.rcvd.set(QName::qxsl("call"), "QV1DOK");
    it.rcvd.set(QName::qxsl("rstq"), "599");
    it.rcvd.set(QName::qxsl("code"), "120103");
    it.sent.set(QName::qxsl("rstq"), "599");
    it.sent.set(QName::qxsl("code"), "100110");
    it
}

fn text(f: &dyn Format, items: &[Item]) -> String {
    encoding_free(&f.write(items).unwrap())
}

#[test]
fn adif_comment_holds_the_note() {
    let mut it = one("14000");
    it.attrs.set(QName::qxsl("note"), "memo text");
    for f in [&Adis as &dyn Format, &Adxs] {
        let out = text(f, std::slice::from_ref(&it));
        assert!(out.contains("COMMENT"), "{out}");
        let back = f.read(out.as_bytes()).unwrap();
        assert_eq!(
            back[0].attrs.qxsl("note"),
            Some("memo text"),
            "{}",
            f.kind()
        );
        assert_eq!(back[0].attrs.adif("COMMENT"), None);
    }
    let plain = text(&Adis, &[one("14000")]);
    assert!(!plain.contains("COMMENT"), "{plain}");
}

#[test]
fn adif_bands_from_24_gigahertz() {
    for (khz, name) in [
        ("24000000", "1.25cm"),
        ("47000000", "6mm"),
        ("77000000", "4mm"),
        ("135000000", "2mm"),
        ("248000000", "1mm"),
    ] {
        let out = text(&Adis, &[one(khz)]);
        assert!(
            out.contains(&format!("<BAND:{}>{name}", name.len())),
            "{out}"
        );
        assert!(!out.contains("FREQ"), "{out}");
        let back = Adis.read(out.as_bytes()).unwrap();
        assert_eq!(back[0].attrs.qxsl("band"), Some(khz), "{name}");
    }
}

#[test]
fn adif_exchange_strings() {
    let mut it = one("14000");
    it.rcvd.set(QName::qxsl("code"), "13M");
    it.sent.set(QName::qxsl("code"), "10H");
    let out = text(&Adis, &[it]);
    assert!(out.contains("<SRX_STRING:3>13M"), "{out}");
    assert!(out.contains("<STX_STRING:3>10H"), "{out}");
    assert!(!out.contains("<SRX:") && !out.contains("<STX:"), "{out}");
    let back = Adis.read(out.as_bytes()).unwrap();
    assert_eq!(back[0].rcvd.qxsl("code"), Some("13M"));
    assert_eq!(back[0].sent.qxsl("code"), Some("10H"));
    let num = text(&Adis, &[one("14000")]);
    assert!(num.contains("<SRX:6>120103") && num.contains("<STX:6>100110"));
    assert!(!num.contains("_STRING"), "{num}");
}

#[test]
fn cabrillo_bands_from_2_4_gigahertz() {
    for (khz, text_band) in [
        ("1200000", "1.2G"),
        ("2400000", "2.3G"),
        ("5600000", "5.7G"),
        ("10000000", "10G"),
        ("24000000", "24G"),
        ("47000000", "47G"),
        ("77000000", "75G"),
        ("135000000", "134G"),
        ("248000000", "241G"),
    ] {
        let out = text(&Cqww, &[one(khz)]);
        assert_eq!(out[5..10].trim(), text_band, "{out}");
        let back = Cqww.read(out.as_bytes()).unwrap();
        // Cabrillo's 75G covers both 75 GHz and 77 GHz; it reads as 75 GHz.
        let want = if khz == "77000000" { "75000000" } else { khz };
        assert_eq!(back[0].attrs.qxsl("band"), Some(want), "{text_band}");
    }
}

#[test]
fn jarl_new_multiplier_points_and_invalid() {
    let mut it = one("14000");
    it.attrs.set(QName::qxsl("mul1"), "13M");
    let plain = text(&Jarl::default(), std::slice::from_ref(&it));
    let line = plain.lines().nth(1).unwrap();
    assert!(line.contains("13M") && line.ends_with("  1"), "{line}");
    let mut old = it.clone();
    old.attrs.set(QName::qxsl("newmult"), "false");
    old.attrs.set(QName::qxsl("points"), "2");
    old.attrs.set(QName::qxsl("invalid"), "true");
    let out = text(&Jarl::default(), &[old.clone()]);
    let line = out.lines().nth(1).unwrap();
    assert!(line.starts_with("X 2017-06-04 09:15"), "{line}");
    assert!(
        !line.contains("13M") && line.ends_with("-        2"),
        "{line}"
    );
    let back = Jarl::default().read(out.as_bytes()).unwrap();
    assert_eq!(back[0].attrs.qxsl("invalid"), Some("true"));
    assert_eq!(back[0].attrs.qxsl("points"), Some("2"));
    assert_eq!(back[0].attrs.qxsl("mul1"), None);
    assert_eq!(back[0].attrs.qxsl("newmult"), None);
    assert_eq!(back[0].rcvd.qxsl("call"), Some("QV1DOK"));
    let mut new = it;
    new.attrs.set(QName::qxsl("newmult"), "true");
    let back = Jarl::default()
        .read(text(&Jarl::default(), &[new]).as_bytes())
        .unwrap();
    assert_eq!(back[0].attrs.qxsl("mul1"), Some("13M"));
    assert_eq!(back[0].attrs.qxsl("newmult"), Some("true"));
    assert_eq!(back[0].attrs.qxsl("points"), Some("1"));
    assert_eq!(back[0].attrs.qxsl("invalid"), None);
}

#[test]
fn cabrillo_invalid_is_x_qso() {
    let mut it = one("14000");
    it.attrs.set(QName::qxsl("invalid"), "true");
    let out = text(&Cqww, &[it, one("7000")]);
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines[0].starts_with("X-QSO: 14000 CW 2017-06-04 0015"),
        "{out}"
    );
    assert!(lines[1].starts_with("QSO:  7000 CW"), "{out}");
    let back = Cqww.read(out.as_bytes()).unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[0].attrs.qxsl("invalid"), Some("true"));
    assert_eq!(back[0].rcvd.qxsl("call"), Some("QV1DOK"));
    assert_eq!(back[1].attrs.qxsl("invalid"), None);
}

/// 10.4 GHz, a frequency in the 10 GHz band, written by each format directly
/// (`encode`, as zlog-qxsl does after `transform`), or through `write` for ADIF.
#[test]
fn ten_point_four_gigahertz() {
    let it = one("10400000");
    for f in [&Adis as &dyn Format, &Adxs] {
        let out = text(f, std::slice::from_ref(&it));
        assert!(out.contains("3cm") && out.contains("10400"), "{out}");
        let raw = f.decode(out.as_bytes()).unwrap();
        assert_eq!(raw[0].attrs.adif("FREQ"), Some("10400"), "{}", f.kind());
    }
    let zall = ZAll::default();
    let out = encoding_free(&zall.encode(std::slice::from_ref(&it)).unwrap());
    assert!(out.contains("10.4GCW"), "{out}");
    let back = zall.decode(out.as_bytes()).unwrap();
    assert_eq!(back[0].attrs.qxsl("band"), Some("10400000"));
    assert_eq!(back[0].attrs.qxsl("mode"), Some("CW"));
    let ctxt = CTxt::default();
    let back = ctxt
        .decode(&ctxt.encode(std::slice::from_ref(&it)).unwrap())
        .unwrap();
    assert_eq!(back[0].attrs.qxsl("band"), Some("10400000"));
    let jarl = Jarl::default();
    let back = jarl
        .decode(&jarl.encode(std::slice::from_ref(&it)).unwrap())
        .unwrap();
    assert_eq!(back[0].attrs.qxsl("band"), Some("10400000"));
}

#[test]
fn cabrillo_122_gigahertz_follows_the_spec() {
    use qxsl::model::{Item, QName};
    let mut item = Item::default();
    item.attrs.set(QName::qxsl("band"), "123000000");
    item.attrs.set(QName::qxsl("mode"), "CW");
    let text = String::from_utf8(Cqww.encode(&[item]).unwrap()).unwrap();
    assert!(text.starts_with("QSO:  122G"), "{text}");
    for band in ["122G", "123G"] {
        let line = text.replacen("122G", band, 1);
        let back = Cqww.decode(line.as_bytes()).unwrap();
        assert_eq!(back[0].attrs.qxsl("band"), Some("123000000"));
    }
}
