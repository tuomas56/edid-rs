#![cfg_attr(feature = "no_std", no_std)]

//! A pure-Rust crate to parse EDID data with `no_std` support. This crate does not include methods for gathering the data from the monitor.
//! 
//! To enable `no_std` support, ensure the `alloc` crate is available, use feature `no_std`, and then implement `edid_rs::Read` instead of `std::io::Read` for data sources.
//! 
//! ### Examples
//! 
//! Basic usage:
//! ```rust
//! extern crate edid_rs;
//! 
//! use std::io::Cursor;
//! 
//! fn main() {
//!     let bytes = vec![...];
//!     println!("{:?}",
//!         edid_rs::parse(&mut Cursor::new(bytes))
//!     );
//! }
//! ```
//! 
//! Reading current monitor EDID on OSX:
//! ```text
//! $ ioreg -l -w0 -d0 -r -c AppleBacklightDisplay | grep IODisplayEDID - | tail -c 258 | head -c 256 | xxd -r -p | cargo run --example stdin
//!    Compiling edid-rs v0.1.0 (../edid)
//!     Finished dev [unoptimized + debuginfo] target(s) in 0.39s
//!      Running `target/debug/examples/stdin`
//! Ok(EDID { product: ProductInformation { manufacturer_id: ManufacturerID('\u{4}', '\u{0}', '\u{6}'), product_code: 40994, serial_number: 0, manufacture_date: ManufactureDate { week: 4, year: 2013 } }, version: Version { version: 1, revision: 4 }, display: DisplayParameters { input: Digital { dfp_compatible: true }, max_size: Some(ImageSize { width: 33.0, height: 21.0 }), gamma: Some(2.2), dpms: DPMSFeatures { standby_supported: false, suspend_supported: false, low_power_supported: false, display_type: Monochrome, default_srgb: false, preferred_timing_mode: true, default_gtf_supported: false } }, color: ColorCharacteristics { red: (0.6533203, 0.33398438), green: (0.2998047, 0.6201172), blue: (0.14648438, 0.049804688), white: (0.3125, 0.32910156), white_points: [] }, timings: Timings { established_timings: [], standard_timings: [], detailed_timings: [DetailedTiming { pixel_clock: 337750000, active: (2880, 1800), front_porch: (48, 3), sync_length: (32, 6), back_porch: (80, 43), image_size: ImageSize { width: 33.1, height: 20.7 }, border: (0, 0), interlaced: false, stereo: None, sync_type: Seperate { horizontal: Positive, vertical: Negative } }] }, descriptors: MonitorDescriptors([MonitorName("Color LCD"), ManufacturerDefined(0, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0])]), extensions: 0 })
//! ```

/// Trait which all data sources must implement. In a `std` environment,
/// there is a blanket impl of `edid_rs::Read` for `std::io::Read`.
pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize>;
}

#[cfg(not(feature = "no_std"))]
impl<T: std::io::Read> Read for T {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        self.read(buf).ok()
    }
}

#[cfg(feature = "no_std")]
#[macro_use]
extern crate alloc;
#[cfg(feature = "no_std")]
use alloc::{vec::Vec, string::String};


/// The type of parsing results.
pub type Result<T> = core::result::Result<T, &'static str>;

// Like `assert!` but returning Err instead of panicking.
fn ensure(pred: bool, msg: &'static str) -> Result<()> {
    if pred {
        Ok(())
    } else {
        Err(msg)
    }
}

/// Used to parse the binary data from a Read value.
pub struct Reader<'a> {
    // The source we are reading from,
    value: &'a mut dyn Read,
    // and a 128-byte buffer of data.
    buffer: Vec<u8>
}

impl<'a> Reader<'a> {
    pub fn new<T: Read>(value: &'a mut T) -> Reader<'a> {
        Reader {
            value: value as &mut dyn Read, buffer: Vec::with_capacity(128)
        }
    }

    // Get one character from the input.
    fn get(&mut self) -> Result<u8> {
        if self.buffer.len() == 0 {
            self.buffer.resize(128, 0);
            let num = self.value.read(self.buffer.as_mut()).ok_or("Error reading data!")?;
            self.buffer.truncate(num);
        }

        if self.buffer.len() > 0 {
            Ok(self.buffer.remove(0))
        } else{
            Err("Unexpectedly out of data!")
        }
    }

    fn read_u8(&mut self) -> Result<u8> {
        self.get()
    }

    // Both this and `read_u32` are little-endian.
    fn read_u16(&mut self) -> Result<u16> {
        Ok((self.read_u8()? as u16) | ((self.read_u8()? as u16) << 8))
    }

    fn read_u32(&mut self) -> Result<u32> {
        Ok((self.read_u16()? as u32) | ((self.read_u16()? as u32) << 16))
    }
}

/// The EDID information block.
#[derive(Debug, Clone)]
pub struct EDID {
    /// Product version information.
    pub product: ProductInformation,
    /// EDID specification version.
    pub version: Version,
    /// Display characteristic parameters.
    pub display: DisplayParameters,
    /// Color calibration parameters.
    pub color: ColorCharacteristics,
    /// Accepted timing modes.
    pub timings: Timings,
    /// Extra monitor information.
    pub descriptors: MonitorDescriptors,
    /// Number of extensions following the EDID block.
    pub extensions: u8,
}

impl EDID {
    pub fn parse(r: &mut Reader) -> Result<EDID> {
        ensure(r.read_u32()? == 0xffffff00, "Invalid header.")?;
        ensure(r.read_u32()? == 0x00ffffff, "Invalid header.")?;
        
        // Parse the different parts of the data,
        let product = ProductInformation::parse(r)?;
        let version = Version::parse(r)?;
        let display = DisplayParameters::parse(r)?;
        let mut color = ColorCharacteristics::parse(r)?;
        let mut timings = Timings::parse(r)?;
        let (descriptors, mut detailed_timings, mut standard_timings, mut white) = MonitorDescriptors::parse(r)?;

        // And do a little rearranging of the monitor descriptors to 
        // put the timing information all in one place.
        color.white_points.append(&mut white);
        timings.detailed_timings.append(&mut detailed_timings);
        timings.standard_timings.append(&mut standard_timings);

        // Finish by reading how many extensions should follow this data.
        // We do not attempt to parse these in any way.
        let extensions = r.read_u8()?;

        Ok(EDID {
            product, version, display, color, timings, descriptors, extensions
        })
    }
}

/// Information about the product and its manufacture.
#[derive(Debug, Clone)]
pub struct ProductInformation {
    pub manufacturer_id: ManufacturerID,
    pub product_code: u16,
    pub serial_number: u32,
    pub manufacture_date: ManufactureDate
}

impl ProductInformation {
    fn parse(r: &mut Reader) -> Result<ProductInformation> {
        let manufacturer_id = ManufacturerID::parse(r)?;
        let product_code = r.read_u16()?;
        let serial_number = r.read_u32()?;
        let manufacture_date = ManufactureDate::parse(r)?;

        Ok(ProductInformation {
            manufacturer_id, product_code, serial_number, manufacture_date
        })
    }
}

/// Three character manufacturer ID.
#[derive(Debug, Clone, Copy)]
pub struct ManufacturerID(pub char, pub char, pub char);

impl ManufacturerID {
    fn parse(r: &mut Reader) -> Result<ManufacturerID> {
        // The manufacturer ID is stored as three 5-bit
        // characters in a 16-bit little endian field.
        let k = r.read_u16()?;
        let c1 = ((k & 0b0111110000000000) >> 10) as u8;
        let c2 = ((k & 0b0000001111100000) >> 05) as u8;
        let c3 = ((k & 0b0000000000011111) >> 00) as u8;
        Ok(ManufacturerID(c1 as char, c2 as char, c3 as char))
    }
}

/// Gregorian calendar date of manufacture, all years are CE.
#[derive(Debug, Clone, Copy)]
pub struct ManufactureDate {
    pub week: u8,
    pub year: u16
}

impl ManufactureDate {
    fn parse(r: &mut Reader) -> Result<ManufactureDate> {
        let week = r.read_u8()?;
        let year = r.read_u8()? as u16 + 1990;

        Ok(ManufactureDate { week, year })
    }
}

/// EDID specification version.
#[derive(Debug, Clone, Copy)]
pub struct Version {
    pub version: u8,
    pub revision: u8
}

impl Version {
    fn parse(r: &mut Reader) -> Result<Version> {
        let version = r.read_u8()?;
        let revision = r.read_u8()?;

        Ok(Version { version, revision })
    }
}

/// Information about the display hardware.
#[derive(Debug, Clone)]
pub struct DisplayParameters {
    pub input: VideoInput,
    /// The maximum size of the image on the monitor.
    pub max_size: Option<ImageSize>,
    /// The display's gamma factor.
    pub gamma: Option<f32>,
    /// DPMS feature support.
    pub dpms: DPMSFeatures
}

impl DisplayParameters {
    fn parse(r: &mut Reader) -> Result<DisplayParameters> {
        let input = VideoInput::parse(r)?;
        let max_width = r.read_u8()?;
        let max_height = r.read_u8()?;

        let max_size = if max_width == 0 || max_height == 0 {
            None
        } else {
            Some(ImageSize {
                width: max_width as f32,
                height: max_height as f32
            })
        };

        let gamma_val = r.read_u8()?;
        let gamma = if gamma_val == 0xff {
            None
        } else {
            Some((gamma_val as f32 + 100.0) / 100.0)
        };

        let dpms = DPMSFeatures::parse(r)?;

        Ok(DisplayParameters { input, max_size, gamma, dpms })
    }   
}

/// Describes the format of the monitors video input.
#[derive(Debug, Clone, Copy)]
pub enum VideoInput {
    Analog {
        /// The video signal voltages.
        signal_level: SignalLevel,
        /// Whether a blank-to-black setup is expected.
        setup_expected: bool,
        /// Which sync signals the monitor supports.
        supported_sync: SupportedSync
    },
    Digital {
        /// Compatible with VESA DFP 1.x
        dfp_compatible: bool
    }
}

impl VideoInput {
    fn parse(r: &mut Reader) -> Result<VideoInput> {
        let val = r.read_u8()?;
        if val & (1 << 7) == 0 {
            let signal_level = match (val & 0b01100000) >> 5 {
                0 => SignalLevel { high: 0.700, low: 0.300 },
                1 => SignalLevel { high: 0.714, low: 0.286 },
                2 => SignalLevel { high: 1.000, low: 0.400 },
                3 => SignalLevel { high: 0.700, low: 0.000 },
                _ => unreachable!()
            };
            let setup_expected = val & (1 << 4) > 0;
            let supported_sync = SupportedSync {
                serrated_vsync: val & (1 << 3) > 0,
                sync_on_green: val & (1 << 2) > 0,
                composite_sync: val & (1 << 1) > 0,
                seperate_sync: val & (1 << 0) > 0
            };
            Ok(VideoInput::Analog { signal_level, setup_expected, supported_sync })
        } else {
            Ok(VideoInput::Digital { dfp_compatible: val & 1 > 0 })
        }
    }
}

/// Gives the minimum and maximum voltages on the video lines.
#[derive(Debug, Clone, Copy)]
pub struct SignalLevel {
    pub high: f32,
    pub low: f32
}

/// Describes what sync signals the monitor accepts.
#[derive(Debug, Clone, Copy)]
pub struct SupportedSync {
    /// HSync during VSync
    pub serrated_vsync: bool,
    /// Sync on just green line or RGB.
    pub sync_on_green: bool,
    /// Sync on HSync line
    pub composite_sync: bool,
    /// Seperate sync signals supported.
    pub seperate_sync: bool
}

/// Image size specified in centimetres.
#[derive(Debug, Clone, Copy)]
pub struct ImageSize {
    pub width: f32,
    pub height: f32
}

/// DPMS features supported by the display.
#[derive(Debug, Clone)]
pub struct DPMSFeatures {
    pub standby_supported: bool,
    pub suspend_supported: bool,
    pub low_power_supported: bool,
    pub display_type: DisplayType,
    pub default_srgb: bool,
    /// If set, the preferred timing mode is specified
    /// in the first detailed timing block. 
    pub preferred_timing_mode: bool,
    /// If set, all timings from the standard GTF will work.
    pub default_gtf_supported: bool
}

impl DPMSFeatures {
    fn parse(r: &mut Reader) -> Result<DPMSFeatures> {
        let val = r.read_u8()?;

        Ok(DPMSFeatures {
            standby_supported: val & (1 << 7) > 0,
            suspend_supported: val & (1 << 6) > 0,
            low_power_supported: val & (1 << 5) > 0,
            display_type: match (val & 0b00011000) >> 3 {
                0 => DisplayType::Monochrome,
                1 => DisplayType::RGBColor,
                2 => DisplayType::OtherColor,
                3 => DisplayType::Undefined,
                _ => unreachable!()
            },
            default_srgb: val & (1 << 2) > 0,
            preferred_timing_mode: val & (1 << 1) > 0,
            default_gtf_supported: val & (1 << 0) > 0
        })
    }
}

/// The type of display.
#[derive(Debug, Clone, Copy)]
pub enum DisplayType {
    Monochrome,
    RGBColor,
    OtherColor,
    Undefined
}

/// Color chromaticity coordinates expressed as CIE 1931 x, y coordinates,
/// as well as additional white points given in the monitor descriptors.
#[derive(Debug, Clone)]
pub struct ColorCharacteristics {
    pub red: (f32, f32),
    pub green: (f32, f32),
    pub blue: (f32, f32),
    pub white: (f32, f32),
    pub white_points: Vec<WhitePoint>
}

impl ColorCharacteristics {
    fn parse(r: &mut Reader) -> Result<ColorCharacteristics> {
        let rg_low = r.read_u8()? as u16;
        let bw_low = r.read_u8()? as u16;
        let rh_x = r.read_u8()? as u16;
        let rh_y = r.read_u8()? as u16;
        let gh_x = r.read_u8()? as u16;
        let gh_y = r.read_u8()? as u16;
        let bh_x = r.read_u8()? as u16;
        let bh_y = r.read_u8()? as u16;
        let wh_x = r.read_u8()? as u16;
        let wh_y = r.read_u8()? as u16;

        let red_x = (rh_x << 2 | (rg_low & 0b11000000) >> 6) as f32 / 1024.0;
        let red_y = (rh_y << 2 | (rg_low & 0b00110000) >> 4) as f32 / 1024.0;
        let green_x = (gh_x << 2 | (rg_low & 0b00001100) >> 2) as f32 / 1024.0;
        let green_y = (gh_y << 2 | (rg_low & 0b00000011) >> 0) as f32 / 1024.0;
        let blue_x = (bh_x << 2 | (bw_low & 0b11000000) >> 6) as f32 / 1024.0;
        let blue_y = (bh_y << 2 | (bw_low & 0b00110000) >> 4) as f32 / 1024.0;
        let white_x = (wh_x << 2 | (bw_low & 0b00001100) >> 2) as f32 / 1024.0;
        let white_y = (wh_y << 2 | (bw_low & 0b00000011) >> 0) as f32 / 1024.0;

        Ok(ColorCharacteristics {
            red: (red_x, red_y),
            green: (green_x, green_y),
            blue: (blue_x, blue_y),
            white: (white_x, white_y),
            white_points: Vec::new()
        })
    }
}

/// A single white point for the display, with x and y
/// chromaticity coordinates given in the CIE 1931 space.
#[derive(Debug, Clone, Copy)]
pub struct WhitePoint {
    pub index: u8,
    pub x: f32,
    pub y: f32,
    pub gamma: f32
}

/// The timing modes accepted by the display.
#[derive(Debug, Clone)]
pub struct Timings {
    /// The timings supported from the VESA 'established timing' list.
    pub established_timings: Vec<EstablishedTiming>,
    /// Standard timings given that can be derived from the GTF.
    pub standard_timings: Vec<StandardTiming>,
    /// Detailed timings specific to the display. If it exists, the first
    /// detailed timing is the preferred timing.
    pub detailed_timings: Vec<DetailedTiming>
}

impl Timings {
    fn parse(r: &mut Reader) -> Result<Timings> {
        let mut ft = r.read_u16()?;

        let mut established_timings = Vec::new();
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H800V600F60);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H800V600F56);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H640V480F75);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H640V480F72);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H640V480F67);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H640V480F60);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H720V400F88);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H720V400F70);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H1280V1024F75);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H1024V768F75);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H1024V768F70);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H1024V768F60);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H1024V768F87);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H832V624F75);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H800V600F75);
        }
        ft >>= 1;
        if ft & 1 > 0 {
            established_timings.push(EstablishedTiming::H800V600F72);
        }

        let ft = r.read_u8()?;
        if ft & (1 << 7) > 0 {
            established_timings.push(EstablishedTiming::H1152V870F75);
        }

        let mut standard_timings = Vec::new();
        for _ in 0..8 {
            let low = r.read_u8()?;
            let high = r.read_u8()?;
            if low == 1 && high == 1 {
                continue;
            } else {
                standard_timings.push(StandardTiming {
                    horizontal_resolution: (low as u16 + 31) * 8,
                    aspect_ratio: match high >> 6 {
                        0 => 16.0/10.0,
                        1 => 4.0/3.0,
                        2 => 5.0/4.0,
                        3 => 16.0/9.0,
                        _ => unreachable!()
                    },
                    refresh_rate: (high & 0b00111111) + 60
                });
            }
        }

        let detailed_timings = Vec::new();

        Ok(Timings { established_timings, standard_timings, detailed_timings })
    }
}

/// The 'established timings' specified by VESA.
#[derive(Debug, Clone, Copy)]
pub enum EstablishedTiming {
    H720V400F70,
    H720V400F88,
    H640V480F60,
    H640V480F67,
    H640V480F72,
    H640V480F75,
    H800V600F56,
    H800V600F60,
    H800V600F72,
    H800V600F75,
    H832V624F75,
    H1024V768F87,
    H1024V768F60,
    H1024V768F70,
    H1024V768F75,
    H1280V1024F75,
    H1152V870F75
}

/// A standard timing which contains enough information to derive the
/// other parameters from the GTF.
#[derive(Debug, Clone, Copy)]
pub struct StandardTiming {
    pub horizontal_resolution: u16,
    pub aspect_ratio: f32,
    pub refresh_rate: u8
}

/// A non-standard timing with all parameters specified.
#[derive(Debug, Clone)]
pub struct DetailedTiming {
    /// Given in Hz
    pub pixel_clock: u32,
    /// Active area in pixels.
    pub active: (u16, u16),
    /// Length of front porch in pixels and lines.
    pub front_porch: (u16, u16),
    /// Length of sync pulse in pixels and lines.
    pub sync_length: (u16, u16),
    /// Length of back porch in pixels and lines.
    pub back_porch: (u16, u16),
    /// Image size in centimetres.
    pub image_size: ImageSize,
    /// Border size in pixels.
    pub border: (u16, u16),
    pub interlaced: bool,
    pub stereo: StereoType,
    pub sync_type: SyncType
}

impl DetailedTiming {
    fn parse(r: &mut Reader) -> Result<Option<DetailedTiming>> {
        let pixel_clock = r.read_u16()? as u32 * 10000;
        let ha_low = r.read_u8()? as u16;

        if pixel_clock == 0 {
            return Ok(None);
        }

        let hb_low = r.read_u8()? as u16;
        let h_high = r.read_u8()? as u16;

        let horizontal_active = ha_low | (((h_high & 0xf0) >> 4) << 8);
        if horizontal_active == 0 {
            return Ok(None);
        }

        let horizontal_blanking = hb_low | (((h_high & 0x0f) >> 0) << 8);

        let va_low = r.read_u8()? as u16;
        let vb_low = r.read_u8()? as u16;
        let v_high = r.read_u8()? as u16;

        let vertical_active = va_low | (((v_high & 0xf0) >> 4) << 8);
        let vertical_blanking = vb_low | (((v_high & 0x0f) >> 0) << 8);

        let hso_low = r.read_u8()? as u16;
        let hsw_low = r.read_u8()? as u16;
        let vs_low = r.read_u8()? as u16;
        let hvs_high = r.read_u8()? as u16;

        let hso_high = (hvs_high & 0b11000000) >> 6;
        let hsw_high = (hvs_high & 0b00110000) >> 4;
        let vso_high = (hvs_high & 0b00001100) >> 2;
        let vsw_high = (hvs_high & 0b00000011) >> 0;
        let vso_low = (vs_low & 0xf0) >> 4;
        let vsw_low = (vs_low & 0x0f) >> 0;
        let vertical_front_porch = vso_low | (vso_high << 4);
        let horizontal_front_porch = hso_low | (hso_high << 8);
        let vertical_sync_width = vsw_low | (vsw_high << 4);
        let horizontal_sync_width = hsw_low | (hsw_high << 8);
        let active = (horizontal_active, vertical_active);
        let front_porch = (horizontal_front_porch, vertical_front_porch);
        let sync_length = (horizontal_sync_width, vertical_sync_width);
        let back_porch = (
            horizontal_blanking - horizontal_sync_width - horizontal_front_porch,
            vertical_blanking - vertical_sync_width - vertical_front_porch
        );

        let hs_low = r.read_u8()? as u16;
        let vs_low = r.read_u8()? as u16;
        let s_high = r.read_u8()? as u16;
        
        let h_size = hs_low | ((s_high & 0xf0) >> 4) << 8;
        let v_size = vs_low | ((s_high & 0x0f) >> 0) << 8;
        let image_size = ImageSize { width: (h_size as f32) / 10.0, height: (v_size as f32) / 10.0 };

        let hb = r.read_u8()? as u16;
        let vb = r.read_u8()? as u16;

        let border = (hb, vb);

        let (interlaced, stereo, sync_type) = SyncType::parse(r)?;

        Ok(Some(DetailedTiming {
            pixel_clock, active, front_porch, sync_length, back_porch, 
            image_size, border, interlaced, stereo, sync_type
        }))
    }
}

/// Type of stereo image supported by the display.
#[derive(Debug, Clone, Copy)]
pub enum StereoType {
    None,
    SequentialRightSync,
    SequentialLeftSync,
    InterleavedLinesRightEven,
    InterleavedLinesLeftEven,
    Interleaved4Way,
    SideBySide
}

/// Sync type for a given timing.
#[derive(Debug, Clone, Copy)]
pub enum SyncType {
    /// Single sync signal.
    Composite {
        /// HSync during VSync
        serrated: bool,
        /// Which line to sync on.
        line: SyncLine
    },
    /// Seperate sync signals.
    Seperate {
        /// Horizontal polarity.
        horizontal: SyncPolarity,
        /// Vertical polarity.
        vertical: SyncPolarity
    }
}

impl SyncType {
    fn parse(r: &mut Reader) -> Result<(bool, StereoType, SyncType)> {
        let val = r.read_u8()?;

        let interlaced = val & (1 << 7) > 0;
        let stereo = match (val & (1 << 6) > 0, val & (1 << 5) > 0, val & (1 << 0) > 0) {
            (false, false, _) => StereoType::None,
            (false, true, false) => StereoType::SequentialRightSync,
            (true, false, false) => StereoType::SequentialLeftSync,
            (false, true, true) => StereoType::InterleavedLinesRightEven,
            (true, false, true) => StereoType::InterleavedLinesLeftEven,
            (true, true, false) => StereoType::Interleaved4Way,
            (true, true, true) => StereoType::SideBySide
        };

        let sync_type = match (val & 0b00011000) >> 3 {
            0 | 1 => SyncType::Composite {
                serrated: val & (1 << 2) > 0,
                line: if val & (1 << 1) > 0 {
                    SyncLine::RGB
                } else {
                    SyncLine::Green
                }
            },
            2 => SyncType::Composite {
                serrated: val & (1 << 2) > 0,
                line: SyncLine::Digital(if val & (1 << 1) > 0 {
                    SyncPolarity::Positive
                } else {
                    SyncPolarity::Negative
                })
            },
            3 => SyncType::Seperate {
                vertical: if val & (1 << 2) > 0 {
                    SyncPolarity::Positive
                } else {
                    SyncPolarity::Negative
                },
                horizontal: if val & (1 << 1) > 0 {
                    SyncPolarity::Positive
                } else {
                    SyncPolarity::Negative
                }
            },
            _ => unreachable!()
        };

        Ok((interlaced, stereo, sync_type))
    }
}

/// A line to perform sync on.
#[derive(Debug, Clone, Copy)]
pub enum SyncLine {
    RGB,
    Green,
    Digital(SyncPolarity)
}

/// The direction of the sync pulse.
#[derive(Debug, Clone, Copy)]
pub enum SyncPolarity {
    Positive,
    Negative
}

/// Additional monitor information.
#[derive(Debug, Clone)]
pub struct MonitorDescriptors(pub Vec<MonitorDescriptor>);

impl MonitorDescriptors {
    fn parse(r: &mut Reader) -> Result<(MonitorDescriptors, Vec<DetailedTiming>, Vec<StandardTiming>, Vec<WhitePoint>)> {
        let mut detailed_timings = vec![DetailedTiming::parse(r)?.ok_or("Expected detailed timing block.")?];

        let mut standard_timings = Vec::new();
        let mut monitor_descriptors = Vec::new();
        let mut white_points = Vec::new();

        for _ in 0..3 {
            if let Some(timing) = DetailedTiming::parse(r)? {
                detailed_timings.push(timing);
            } else {
                let tag = r.read_u8()?;
                r.read_u8()?;

                match tag {
                    0x00..=0x0f => monitor_descriptors.push(MonitorDescriptor::ManufacturerDefined(tag, [
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?
                    ])),
                    0x10 => continue,
                    0x11..=0xf9 => monitor_descriptors.push(MonitorDescriptor::Undefined(tag, [
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?,
                        r.read_u8()?
                    ])),
                    0xfa => {
                        for _ in 0..6 {
                            let low = r.read_u8()?;
                            let high = r.read_u8()?;
                            if low == 1 && high == 1 {
                                continue;
                            } else {
                                standard_timings.push(StandardTiming {
                                    horizontal_resolution: (low as u16 + 31) * 8,
                                    aspect_ratio: match high >> 6 {
                                        0 => 16.0/10.0,
                                        1 => 4.0/3.0,
                                        2 => 5.0/4.0,
                                        3 => 16.0/9.0,
                                        _ => unreachable!()
                                    },
                                    refresh_rate: (high & 0b00111111) + 60
                                });
                            }
                        }

                        ensure(r.read_u8()? == 0x0a, "Expected 0x0a in monitor descriptor.")?;
                    },
                    0xfb => {
                        for _ in 0..2 {
                            let index = r.read_u8()?;
                            let w_low = r.read_u8()? as u16; 
                            let wx_high = r.read_u8()? as u16;
                            let wy_high = r.read_u8()? as u16;
                            let white_x = (wx_high << 2 | (w_low & 0b00001100) >> 2) as f32 / 1024.0;
                            let white_y = (wy_high << 2 | (w_low & 0b00000011) >> 0) as f32 / 1024.0;
                            let gamma_val = r.read_u8()? as u16;
                            let gamma = (gamma_val as f32 + 100.0) / 100.0;
                            white_points.push(WhitePoint { x: white_x, y: white_y, gamma, index });
                            if index == 0 {
                                r.read_u32()?;
                                r.read_u8()?;
                                break;
                            }
                        }

                        ensure(r.read_u8()? == 0x0a, "Expected 0x0a in monitor descriptor.")?;
                        ensure(r.read_u16()? == 0x2020, "Expected 0x20 in monitor descriptor.")?;
                    },
                    0xfc | 0xfe | 0xff => {
                        let mut out = String::new();
                        let mut byte = r.read_u8()?;
                        let mut i = 0;
                        while byte != 0x0a {
                            out.push(byte as char);
                            i += 1;
                            if i == 13 {
                                break;
                            }
                            byte = r.read_u8()?;
                        }
                        i += 1;
                        while i < 13 {
                            ensure(r.read_u8()? == 0x20, "Expected 0x20 in monitor descriptor.")?;
                            i += 1;
                        }

                        match tag {
                            0xfc => monitor_descriptors.push(MonitorDescriptor::MonitorName(out)),
                            0xfe => monitor_descriptors.push(MonitorDescriptor::OtherString(out)),
                            0xff => monitor_descriptors.push(MonitorDescriptor::SerialNumber(out)),
                            _ => unreachable!()
                        }
                    },
                    0xfd => {
                        let min_vrate = r.read_u8()?;
                        let max_vrate = r.read_u8()?;
                        let min_hrate = r.read_u8()? as u32 * 1000;
                        let max_hrate = r.read_u8()? as u32 * 1000;
                        let pixel_clock = r.read_u8()? as u32 * 10000000;
                        let stime = r.read_u8()?;
                        let secondary_timing = match stime {
                            0x00 => {
                                ensure(r.read_u8()? == 0x0a, "Expected 0x0a in monitor descriptor.")?;
                                ensure(r.read_u16()? == 0x2020, "Expected 0x20 in monitor descriptor.")?;
                                ensure(r.read_u16()? == 0x2020, "Expected 0x20 in monitor descriptor.")?;
                                ensure(r.read_u16()? == 0x2020, "Expected 0x20 in monitor descriptor.")?;
                                SecondaryTiming::None
                            },
                            0x02 => {
                                ensure(r.read_u8()? == 0x00, "Expected 0x0a in monitor descriptor.")?;
                                let start_horizontal_freq = r.read_u8()? as u32 * 2000;
                                let c = r.read_u8()? as f32 / 2.0;
                                let m = r.read_u16()? as f32;
                                let k = r.read_u8()? as f32;
                                let j = r.read_u8()? as f32 / 2.0;
                                SecondaryTiming::GTF {
                                    start_horizontal_freq, c, m, k, j
                                }
                            },
                            _ => {
                                let data = [
                                    r.read_u8()?,
                                    r.read_u8()?,
                                    r.read_u8()?,
                                    r.read_u8()?,
                                    r.read_u8()?,
                                    r.read_u8()?,
                                    r.read_u8()?
                                ];
                                SecondaryTiming::Other(stime, data)
                            }
                        };
                        monitor_descriptors.push(MonitorDescriptor::RangeLimits {
                            vertical_rate: (min_vrate, max_vrate),
                            horizontal_rate: (min_hrate, max_hrate),
                            pixel_clock, secondary_timing
                        });
                    }
                }
            }
        }

        Ok((MonitorDescriptors(monitor_descriptors), detailed_timings, standard_timings, white_points))
    }
}

/// One piece of additional monitor information.
#[derive(Debug, Clone)]
pub enum MonitorDescriptor {
    SerialNumber(String),
    OtherString(String),
    RangeLimits {
        /// Vertical frequency limits in Hz.
        vertical_rate: (u8, u8),
        /// Horizontal frequency limits in Hz.
        horizontal_rate: (u32, u32),
        /// Pixel frequency limits in Hz.
        pixel_clock: u32,
        /// Seconday timing information.
        secondary_timing: SecondaryTiming
    },
    MonitorName(String),
    Undefined(u8, [u8; 13]),
    ManufacturerDefined(u8, [u8; 13])
}

/// Parameters for a secondary timing formula.
#[derive(Debug, Clone)]
pub enum SecondaryTiming {
    None,
    /// Alternative GTF parameters.
    GTF {
        /// Horizontal frequency from which this applies.
        start_horizontal_freq: u32,
        c: f32,
        m: f32,
        k: f32,
        j: f32
    },
    Other(u8, [u8; 7])
}

/// Parse EDID data from a Read value.
pub fn parse<T: Read + 'static>(value: &mut T) -> Result<EDID> {
    EDID::parse(&mut Reader::new(value))
}
