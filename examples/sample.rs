extern crate edid_rs;

// Sample EDID data from a Macbook Pro.
// (Precisely a MacBookPro 11,3 'i7 2.6')
const BYTES: [u8; 128] = [
      0, 255, 255, 255, 255, 255, 255,   0,
      6,  16,  34, 160,   0,   0,   0,   0,
      4,  23,   1,   4, 165,  33,  21, 120,
      2, 111, 177, 167,  85,  76, 158,  37,
     12,  80,  84,   0,   0,   0,   1,   1,
      1,   1,   1,   1,   1,   1,   1,   1,
      1,   1,   1,   1,   1,   1, 239, 131,
     64, 160, 176,   8,  52, 112,  48,  32,
     54,   0,  75, 207,  16,   0,   0,  26,
      0,   0,   0, 252,   0,  67, 111, 108, 
    111, 114,  32,  76,  67,  68,  10,  32, 
     32,  32,   0,   0,   0,  16,   0,   0, 
      0,   0,   0,   0,   0,   0,   0,   0, 
      0,   0,   0,   0,   0,   0,   0,  16,
      0,   0,   0,   0,   0,   0,   0,   0,
      0,   0,   0,   0,   0,   0,   0, 222
];

fn main() {
    println!("{:#?}", edid_rs::parse(&mut std::io::Cursor::new(&BYTES[..])));
}