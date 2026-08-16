//! AVTX — Avalanche texture container. 0x80-byte header + inline mip data (smaller mips inline; the
//! largest mips may live in an external hi-res `.hmddsc` stream). Cracked double-blind (both
//! investigators converged; 1193/1193 oracle). Decomp: writer FUN_14aa1bfd0, magic FUN_14aa1c0a0.
//! See ../../docs/formats/avtx.md.
//!
//! Header (little-endian):
//!   0x00 char[4] "AVTX"          0x08 u32 dxgi_format     0x14 u8 mip_total (full chain)
//!   0x04 u16 version=1           0x0C u16 width           0x15 u8 mip_inline (in this file)
//!   0x06 u8 unknown              0x0E u16 height          0x20 u32 size_header = 0x80
//!   0x07 u8 dimension (2=2D,3=3D)0x10 u16 depth/slices     0x24 u32 size_body = filelen-0x80
//!   0x12 u16 flags (0x40=cubemap, 0x01=external mips)      0x28 u32 = 0x10 (const)
//!   pixel data begins at 0x80; inline mips stored largest-first (per slice/face).

const MAGIC: u32 = 0x58545641; // "AVTX"
pub const HEADER_LEN: usize = 0x80;

fn u16le(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }

pub struct AvtxHeader {
    pub version: u16,
    pub dimension: u8,
    pub dxgi_format: u32,
    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub flags: u16,
    pub mip_total: u8,
    pub mip_inline: u8,
    pub size_body: u32,
}

impl AvtxHeader {
    pub fn is_cubemap(&self) -> bool { self.flags & 0x40 != 0 }
    pub fn has_external_mips(&self) -> bool { self.mip_inline < self.mip_total }
}

pub fn parse_header(b: &[u8]) -> Result<AvtxHeader, String> {
    if b.len() < HEADER_LEN || u32le(b, 0) != MAGIC {
        return Err("not an AVTX (bad magic)".into());
    }
    Ok(AvtxHeader {
        version: u16le(b, 4),
        dimension: b[7],
        dxgi_format: u32le(b, 8),
        width: u16le(b, 0x0C),
        height: u16le(b, 0x0E),
        depth: u16le(b, 0x10),
        flags: u16le(b, 0x12),
        mip_total: b[0x14],
        mip_inline: b[0x15],
        size_body: u32le(b, 0x24),
    })
}

/// (block_w, block_h, block_bytes) for a DXGI format; None if unsupported.
pub fn block_info(dxgi: u32) -> Option<(u32, u32, u32)> {
    Some(match dxgi {
        71 => (4, 4, 8),   // BC1_UNORM
        74 => (4, 4, 16),  // BC2_UNORM
        77 => (4, 4, 16),  // BC3_UNORM
        80 => (4, 4, 8),   // BC4_UNORM
        83 => (4, 4, 16),  // BC5_UNORM
        98 => (4, 4, 16),  // BC7_UNORM
        87 | 28 => (1, 1, 4),  // B8G8R8A8 / R8G8B8A8
        10 => (1, 1, 8),   // R16G16B16A16_FLOAT
        26 => (1, 1, 4),   // R11G11B10_FLOAT
        _ => return None,
    })
}

/// Byte size of one mip surface (single slice/face) at `w×h` for `dxgi`.
pub fn mip_size(w: u32, h: u32, dxgi: u32) -> Option<usize> {
    let (bw, bh, bb) = block_info(dxgi)?;
    Some((w.div_ceil(bw) * h.div_ceil(bh) * bb) as usize)
}

pub fn dxgi_name(dxgi: u32) -> &'static str {
    match dxgi {
        71 => "BC1_UNORM", 74 => "BC2_UNORM", 77 => "BC3_UNORM", 80 => "BC4_UNORM",
        83 => "BC5_UNORM", 98 => "BC7_UNORM", 87 => "B8G8R8A8_UNORM", 28 => "R8G8B8A8_UNORM",
        10 => "R16G16B16A16_FLOAT", 26 => "R11G11B10_FLOAT", _ => "UNKNOWN",
    }
}

/// Expected inline byte count = slices × Σ mip_size over the inline mip levels. Equals `size_body`
/// (and `filelen - 0x80`) on all retail AVTX. slices = depth × (6 if cubemap). NOTE: treats `depth` as
/// a constant per-mip slice count (2D-array/cubemap); true 3D volumes (mip depth halving) are untested.
pub fn expected_inline_size(h: &AvtxHeader) -> Option<usize> {
    let faces = if h.is_cubemap() { 6 } else { 1 };
    let slices = (h.depth.max(1) as usize) * faces;
    let start = h.mip_total.saturating_sub(h.mip_inline);
    let mut sum = 0usize;
    for i in start..h.mip_total {
        let w = (h.width as u32 >> i).max(1);
        let ht = (h.height as u32 >> i).max(1);
        sum += mip_size(w, ht, h.dxgi_format)?;
    }
    Some(sum * slices)
}

/// A single mip surface: dimensions, DXGI format, and its (possibly block-compressed) bytes.
pub struct Mip {
    pub width: u32,
    pub height: u32,
    pub dxgi_format: u32,
    pub data: Vec<u8>,
}

/// The largest mip actually present in this AVTX (slice/face 0), for rendering. When the base mip is
/// external (`has_external_mips`), this is the largest *inline* mip — the best obtainable from the file
/// alone. It sits first at offset 0x80 (inline mips are largest-first).
pub fn best_inline_mip(b: &[u8]) -> Result<Mip, String> {
    let h = parse_header(b)?;
    let start = (h.mip_total.saturating_sub(h.mip_inline)) as u32;
    let w = (h.width as u32 >> start).max(1);
    let ht = (h.height as u32 >> start).max(1);
    let size = mip_size(w, ht, h.dxgi_format)
        .ok_or_else(|| format!("unsupported dxgi_format {:#x}", h.dxgi_format))?;
    if HEADER_LEN + size > b.len() {
        return Err(format!("mip {w}x{ht} ({size} B) overruns file ({} B)", b.len()));
    }
    Ok(Mip { width: w, height: ht, dxgi_format: h.dxgi_format, data: b[HEADER_LEN..HEADER_LEN + size].to_vec() })
}

/// Decode a mip surface to tightly-packed RGBA8 (`width*height*4` bytes) for display. Block formats go
/// through `texture2ddecoder`; the two uncompressed formats are copied/swizzled. `texture2ddecoder`
/// emits `0xAARRGGBB` u32s (channel order locked by the `bc1_red` test). Returns Err for formats we
/// don't rasterize yet (the float HDR formats).
pub fn decode_rgba(m: &Mip) -> Result<Vec<u8>, String> {
    let (w, h) = (m.width as usize, m.height as usize);
    let n = w * h;
    let mut img = vec![0u32; n];
    type Dec = fn(&[u8], usize, usize, &mut [u32]) -> Result<(), &'static str>;
    let run = |f: Dec, img: &mut [u32]| f(&m.data, w, h, img).map_err(|e| e.to_string());
    match m.dxgi_format {
        71 => run(texture2ddecoder::decode_bc1, &mut img)?,
        74 => run(texture2ddecoder::decode_bc2, &mut img)?,
        77 => run(texture2ddecoder::decode_bc3, &mut img)?,
        80 => run(texture2ddecoder::decode_bc4, &mut img)?,
        83 => run(texture2ddecoder::decode_bc5, &mut img)?,
        98 => run(texture2ddecoder::decode_bc7, &mut img)?,
        28 => { // R8G8B8A8_UNORM — already RGBA8
            if m.data.len() < n * 4 { return Err("truncated R8G8B8A8 surface".into()); }
            return Ok(m.data[..n * 4].to_vec());
        }
        87 => { // B8G8R8A8_UNORM — swizzle B<->R
            if m.data.len() < n * 4 { return Err("truncated B8G8R8A8 surface".into()); }
            let mut out = m.data[..n * 4].to_vec();
            for px in out.chunks_exact_mut(4) { px.swap(0, 2); }
            return Ok(out);
        }
        f => return Err(format!("dxgi {f} ({}) not rasterized yet", dxgi_name(f))),
    }
    let mut out = Vec::with_capacity(n * 4);
    for &p in &img {
        out.push((p >> 16) as u8); // R
        out.push((p >> 8) as u8);  // G
        out.push(p as u8);         // B
        out.push((p >> 24) as u8); // A
    }
    Ok(out)
}

/// Wrap a mip in a DDS container (DX10 extended header, carries the DXGI format verbatim) so it opens
/// in any DDS viewer / community tool.
pub fn to_dds(m: &Mip) -> Vec<u8> {
    fn put(buf: &mut [u8], o: usize, v: u32) { buf[o..o + 4].copy_from_slice(&v.to_le_bytes()); }
    let mut out = Vec::with_capacity(0x80 + m.data.len());
    out.extend_from_slice(b"DDS ");
    let mut hdr = [0u8; 124];
    put(&mut hdr, 0, 124);                                  // dwSize
    put(&mut hdr, 4, 0x1 | 0x2 | 0x4 | 0x1000 | 0x80000);  // CAPS|HEIGHT|WIDTH|PIXELFORMAT|LINEARSIZE
    put(&mut hdr, 8, m.height);
    put(&mut hdr, 12, m.width);
    put(&mut hdr, 16, m.data.len() as u32);                // pitchOrLinearSize
    put(&mut hdr, 20, 1);                                   // depth
    put(&mut hdr, 24, 1);                                   // mipmapcount
    put(&mut hdr, 72, 32);                                  // ddspf.dwSize
    put(&mut hdr, 76, 0x4);                                 // ddspf.dwFlags = DDPF_FOURCC
    hdr[80..84].copy_from_slice(b"DX10");                   // ddspf.dwFourCC
    put(&mut hdr, 104, 0x1000);                             // dwCaps = TEXTURE
    out.extend_from_slice(&hdr);
    let mut dx10 = [0u8; 20];
    put(&mut dx10, 0, m.dxgi_format);
    put(&mut dx10, 4, 3);                                   // resourceDimension = TEXTURE2D
    put(&mut dx10, 12, 1);                                  // arraySize
    out.extend_from_slice(&dx10);
    out.extend_from_slice(&m.data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    // Minimal BC1 4x4 AVTX (mirrors real sample 03533776.avtx): 0x80 header + 8 bytes of block data.
    fn tiny_bc1() -> Vec<u8> {
        let mut b = vec![0u8; HEADER_LEN + 8];
        b[0..4].copy_from_slice(b"AVTX");
        b[4..6].copy_from_slice(&1u16.to_le_bytes());  // version
        b[7] = 2;                                       // dimension 2D
        b[8..12].copy_from_slice(&71u32.to_le_bytes()); // BC1
        b[0x0C..0x0E].copy_from_slice(&4u16.to_le_bytes());
        b[0x0E..0x10].copy_from_slice(&4u16.to_le_bytes());
        b[0x10..0x12].copy_from_slice(&1u16.to_le_bytes());
        b[0x14] = 1; b[0x15] = 1;                       // mip_total / mip_inline
        b[0x20..0x24].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x24..0x28].copy_from_slice(&8u32.to_le_bytes());
        b
    }
    #[test]
    fn parse_and_extract() {
        let b = tiny_bc1();
        let h = parse_header(&b).unwrap();
        assert_eq!(h.dxgi_format, 71);
        assert_eq!((h.width, h.height), (4, 4));
        let m = best_inline_mip(&b).unwrap();
        assert_eq!((m.width, m.height), (4, 4));
        assert_eq!(m.data.len(), 8);            // BC1 4x4 = 8 bytes
        let dds = to_dds(&m);
        assert_eq!(&dds[0..4], b"DDS ");
        assert_eq!(dds.len(), 4 + 124 + 20 + 8);
    }
    #[test]
    fn block_math() {
        assert_eq!(mip_size(4, 4, 71), Some(8));      // BC1 4x4
        assert_eq!(mip_size(128, 128, 77), Some(16384)); // BC3 128x128
    }
    #[test]
    fn bc1_red() {
        // BC1 block: color0 = RGB565 red (0xF800), color1 = 0, all indices 0 -> every texel = color0.
        // Locks texture2ddecoder's channel order to our 0xAARRGGBB extraction (R high, G/B low, A=255).
        let block = [0x00u8, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let m = Mip { width: 4, height: 4, dxgi_format: 71, data: block.to_vec() };
        let rgba = decode_rgba(&m).unwrap();
        assert_eq!(rgba.len(), 4 * 4 * 4);
        let (r, g, b, a) = (rgba[0], rgba[1], rgba[2], rgba[3]);
        assert!(r > 240 && g < 16 && b < 16 && a == 255, "expected opaque red, got ({r},{g},{b},{a})");
    }
}
