//! Minimal PNG encoder from scratch.
//! Produces valid PNG files with IHDR, IDAT, and IEND chunks.

use crate::deflate;

/// CRC32 lookup table (standard polynomial 0xEDB88320)
const fn make_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut n = 0usize;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            if c & 1 != 0 {
                c = 0xEDB88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = make_crc_table();

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &b in data {
        crc = CRC_TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFFFFFF
}

fn write_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    let len = data.len() as u32;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);
    // CRC covers chunk type + data
    let mut crc_data = Vec::with_capacity(4 + data.len());
    crc_data.extend_from_slice(chunk_type);
    crc_data.extend_from_slice(data);
    let crc = crc32(&crc_data);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Encode a 256x256 RGB pixel buffer as a PNG file.
/// `pixels` must be exactly 256*256*3 = 196608 bytes (row-major RGB).
pub fn encode_png(pixels: &[u8]) -> Vec<u8> {
    assert_eq!(pixels.len(), 256 * 256 * 3);

    let mut out = Vec::with_capacity(200_000);

    // PNG signature
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR chunk
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&256u32.to_be_bytes()); // width
    ihdr.extend_from_slice(&256u32.to_be_bytes()); // height
    ihdr.push(8);  // bit depth
    ihdr.push(2);  // color type: RGB
    ihdr.push(0);  // compression method
    ihdr.push(0);  // filter method
    ihdr.push(0);  // interlace method
    write_chunk(&mut out, b"IHDR", &ihdr);

    // Prepare raw image data with filter bytes
    // Each row: 1 filter byte (0 = None) + 256*3 pixel bytes
    let row_len = 1 + 256 * 3;
    let mut raw = Vec::with_capacity(256 * row_len);
    for y in 0..256 {
        raw.push(0); // filter type: None
        let start = y * 256 * 3;
        raw.extend_from_slice(&pixels[start..start + 256 * 3]);
    }

    // Compress with zlib stored blocks
    let compressed = deflate::zlib_stored(&raw);
    write_chunk(&mut out, b"IDAT", &compressed);

    // IEND chunk
    write_chunk(&mut out, b"IEND", &[]);

    out
}
