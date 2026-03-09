//! Minimal DEFLATE/zlib implementation using stored blocks (no compression).
//! This is the fastest possible approach — just wraps raw data in the DEFLATE
//! stored-block format with a zlib header.

/// Wrap raw data in zlib format using DEFLATE stored blocks (type 0).
/// No actual compression — maximum speed.
pub fn zlib_stored(data: &[u8]) -> Vec<u8> {
    // zlib header: CMF=0x78 (deflate, window=32768), FLG=0x01 (check bits)
    // CMF = 0x78: CM=8 (deflate), CINFO=7 (32K window)
    // FLG: FCHECK must make (CMF*256 + FLG) % 31 == 0
    // 0x78 * 256 + 0x01 = 30721; 30721 % 31 = 0. 
    let mut out = Vec::with_capacity(data.len() + 64);
    out.push(0x78);
    out.push(0x01);

    // Split into stored blocks of max 65535 bytes
    let max_block = 65535usize;
    let mut offset = 0;
    while offset < data.len() {
        let remaining = data.len() - offset;
        let block_len = remaining.min(max_block);
        let is_final = offset + block_len >= data.len();
        // BFINAL (1 bit) + BTYPE=00 (2 bits), packed in a byte
        out.push(if is_final { 0x01 } else { 0x00 });
        let len = block_len as u16;
        let nlen = !len;
        out.push((len & 0xFF) as u8);
        out.push((len >> 8) as u8);
        out.push((nlen & 0xFF) as u8);
        out.push((nlen >> 8) as u8);
        out.extend_from_slice(&data[offset..offset + block_len]);
        offset += block_len;
    }

    // Handle empty data
    if data.is_empty() {
        out.push(0x01); // final block
        out.push(0x00);
        out.push(0x00);
        out.push(0xFF);
        out.push(0xFF);
    }

    // Adler-32 checksum of original data
    let checksum = adler32(data);
    out.push((checksum >> 24) as u8);
    out.push((checksum >> 16) as u8);
    out.push((checksum >> 8) as u8);
    out.push(checksum as u8);

    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}
