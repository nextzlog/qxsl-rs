qxsl-rs
====

![image](https://img.shields.io/badge/Rust-2021-red.svg)
![image](https://img.shields.io/badge/license-MIT-darkblue.svg)

qxsl-rs is a Rust library that reads and writes amateur-radio log files, ported from the log I/O of [QxSL](https://github.com/autodyne/qxsl).
Each format is decoded into one model, a list of items whose attributes are namespaced as in QXML, and encoded back from it, so attributes a format does not know are kept.
Each item has a received and a sent node: the callsign `call` of the other station is in the received node, and that of the own station in the sent node.
Formats with a field for the own callsign (QXML, ADIF `STATION_CALLSIGN`, Cabrillo) carry the sent `call`; the others leave it unset on reading and ignore it on writing.

## Formats

| Module | Format |
|---|---|
| `adis`, `adxs` | ADIF (ADI, ADX) |
| `cqww` | Cabrillo |
| `jarl`, `jswl` | JARL electronic log, JARL SWL log |
| `qxml` | QXML |
| `zall`, `zdos` | zLog ALL and DOS text |
| `ctxt`, `cbin` | CTESTWIN text and LG8 |
| `hbin` | Turbo HAMLOG HDB |

The memo `note` is ADIF `COMMENT`; an exchange that is not an integer is ADIF `SRX_STRING` and `STX_STRING`.
The item attributes `points`, `newmult` and `invalid` are carried by the JARL electronic log (points, multiplier column, `X` mark) and `invalid` also by Cabrillo (`X-QSO:`).
A frequency other than its band's nominal one, such as 10.4 GHz, is written as ADIF `FREQ` besides `BAND`; the zLog, JARL, CTESTWIN text and HAMLOG formats write 10.4 GHz in their band columns, while Cabrillo (`10G` for both) and CTESTWIN LG8 (no band index) cannot tell it from 10 GHz.

## Usage

```toml
[dependencies]
qxsl = { git = "https://github.com/nextzlog/qxsl-rs" }
```

## License

### Author

[無線部開発班](https://nextzlog.dev)

### Clauses

- This library is released under the [MIT License](LICENSE).
- It is ported from QxSL, released under the GNU Lesser General Public License, by permission of its author; see [NOTICE](NOTICE.md).
