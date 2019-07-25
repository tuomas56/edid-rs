# `edid-rs`

[![edid-rs](https://docs.rs/edid-rs/badge.svg)](https://docs.rs/edid-rs/0.1.0/edid_rs/)
[![edid-rs](https://img.shields.io/crates/v/edid-rs.svg)](https://crates.io/crates/edid-rs/)

A pure-Rust crate to parse EDID data with `no_std` support. This crate does not include methods for gathering the data from the monitor.

To enable `no_std` support, ensure the `alloc` crate is available, use feature `no_std`, and then implement `edid_rs::Read` instead of `std::io::Read` for data sources.

Dual licensed under MIT and Apache-2.0.

### Examples

Basic usage:
```rust
extern crate edid_rs;

use std::io::Cursor;

fn main() {
    let bytes = vec![...];
    println!("{:?}",
        edid_rs::parse(&mut Cursor::new(bytes))
    );
}
```

Reading current monitor EDID on OSX:
```
$ ioreg -l -w0 -d0 -r -c AppleBacklightDisplay | grep IODisplayEDID - | tail -c 258 | head -c 256 | xxd -r -p | cargo run --example stdin
   Compiling edid-rs v0.1.0 (../edid)
    Finished dev [unoptimized + debuginfo] target(s) in 0.39s
     Running `target/debug/examples/stdin`
Ok(EDID { product: ProductInformation { manufacturer_id: ManufacturerID('\u{4}', '\u{0}', '\u{6}'), product_code: 40994, serial_number: 0, manufacture_date: ManufactureDate { week: 4, year: 2013 } }, version: Version { version: 1, revision: 4 }, display: DisplayParameters { input: Digital { dfp_compatible: true }, max_size: Some(ImageSize { width: 33.0, height: 21.0 }), gamma: Some(2.2), dpms: DPMSFeatures { standby_supported: false, suspend_supported: false, low_power_supported: false, display_type: Monochrome, default_srgb: false, preferred_timing_mode: true, default_gtf_supported: false } }, color: ColorCharacteristics { red: (0.6533203, 0.33398438), green: (0.2998047, 0.6201172), blue: (0.14648438, 0.049804688), white: (0.3125, 0.32910156), white_points: [] }, timings: Timings { established_timings: [], standard_timings: [], detailed_timings: [DetailedTiming { pixel_clock: 337750000, active: (2880, 1800), front_porch: (48, 3), sync_length: (32, 6), back_porch: (80, 43), image_size: ImageSize { width: 33.1, height: 20.7 }, border: (0, 0), interlaced: false, stereo: None, sync_type: Seperate { horizontal: Positive, vertical: Negative } }] }, descriptors: MonitorDescriptors([MonitorName("Color LCD"), ManufacturerDefined(0, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0])]), extensions: 0 })

```
