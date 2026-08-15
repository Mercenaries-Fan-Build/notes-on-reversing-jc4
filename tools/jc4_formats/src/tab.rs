//! TAB/ARC v2 archive: 24-byte header + compression-block table + 20-byte entries; payloads are
//! stored / zlib / Oodle. Cracked double-blind; matches gibbed `ArchiveTableFile.cs` bit-for-bit and
//! verified on all 159 retail tabs. See ../../docs/formats/tab_arc_v2.md.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use crate::oodle::Oodle;

pub const GAME_DIR: &str = "C:/Program Files (x86)/Steam/steamapps/common/Just Cause 4";

/// Oodle DLL path: `$JC4_OODLE_DLL` override, else the game dir's `oo2core_7_win64.dll`.
pub fn default_oodle_dll() -> String {
    std::env::var("JC4_OODLE_DLL").unwrap_or_else(|_| format!("{GAME_DIR}/oo2core_7_win64.dll"))
}

fn u16le(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }

const HDR_LEN: usize = 0x18; // 24-byte fixed header

pub struct Header {
    pub version_major: u16,
    pub version_minor: u16,
    pub alignment: u32,
    pub max_comp_block_size: u32,
    pub block_uncompressed_size: u32,
    pub block_count: u32,
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub name_hash: u32,
    pub offset: u32,
    pub comp_size: u32,
    pub uncomp_size: u32,
    pub flags: u32,
}

impl Entry {
    /// flags[15:0] — first compression-block index (gibbed `CompressedBlockIndex`).
    pub fn first_block(&self) -> u16 { (self.flags & 0xFFFF) as u16 }
    /// flags[23:16] — codec id (gibbed `CompressionType`): 0=raw 1=zlib 4=Oodle.
    pub fn codec(&self) -> u8 { ((self.flags >> 16) & 0xFF) as u8 }
    /// flags[24] — gibbed `CompressionFlags` bit 0 ("Unknown"); not used for decode (name inferred).
    pub fn multi_block(&self) -> bool { (self.flags >> 24) & 1 == 1 }
    pub fn codec_name(&self) -> &'static str {
        match self.codec() {
            0 => "raw", 1 => "zlib", 2 => "lz4", 3 => "zstd", 4 => "oodle", _ => "?",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Block {
    pub comp_size: u32,
    pub uncomp_size: u32,
}
impl Block {
    /// `(0xFFFFFFFF, 0xFFFFFFFF)` = DECA's `no_block` separator (no arc payload).
    pub fn is_sentinel(&self) -> bool { self.comp_size == 0xFFFFFFFF && self.uncomp_size == 0xFFFFFFFF }
}

pub struct Tab {
    pub header: Header,
    pub blocks: Vec<Block>,
    pub entries_off: usize,
    pub entries: Vec<Entry>,
    pub trailing: usize, // bytes left over after the entry table (must be 0)
}

pub fn parse_tab(b: &[u8]) -> Result<Tab, String> {
    if b.len() < HDR_LEN || &b[0..4] != b"TAB\0" {
        return Err("not a TAB file (bad magic)".into());
    }
    let header = Header {
        version_major: u16le(b, 4),
        version_minor: u16le(b, 6),
        alignment: u32le(b, 8),
        max_comp_block_size: u32le(b, 0x10),
        block_uncompressed_size: u32le(b, 0x14),
        block_count: u32le(b, 0x18),
    };
    let entries_off = 0x1C + header.block_count as usize * 8;
    if entries_off > b.len() {
        return Err(format!("block table overruns file (entries_off {entries_off} > {})", b.len()));
    }
    let mut blocks = Vec::with_capacity(header.block_count as usize);
    for i in 0..header.block_count as usize {
        let o = 0x1C + i * 8;
        blocks.push(Block { comp_size: u32le(b, o), uncomp_size: u32le(b, o + 4) });
    }
    let region = b.len() - entries_off;
    let n = region / 20;
    let trailing = region % 20;
    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let o = entries_off + i * 20;
        entries.push(Entry {
            name_hash: u32le(b, o),
            offset: u32le(b, o + 4),
            comp_size: u32le(b, o + 8),
            uncomp_size: u32le(b, o + 12),
            flags: u32le(b, o + 16),
        });
    }
    Ok(Tab { header, blocks, entries_off, entries, trailing })
}

/// Sniff a decoded payload's magic → file extension (for naming/type dispatch).
pub fn magic_ext(b: &[u8]) -> &'static str {
    if b.len() >= 8 && u32le(b, 0) == 4 && &b[4..8] == b"SARC" { return "sarc"; }
    if b.len() >= 4 {
        match &b[0..4] {
            b" FDA" | b"ADF\0" => return "adf",
            b"DDS " => return "dds",
            b"AVTX" => return "avtx",
            b"AAF\0" => return "aaf",
            b"FSB5" | b"FSB4" => return "fsb",
            b"RIFF" => return "wav",
            b"RBMD" => return "rbm",
            b"RTPC" => return "rtpc",
            _ => {}
        }
    }
    "bin"
}

/// Decompress one buffer by codec id (0=raw/None, 1=zlib, 4=Oodle) — matches gibbed's
/// GetDecompress(CompressionType), so per-block entries dispatch on codec instead of assuming Oodle.
pub fn decompress_codec(codec: u8, comp: &[u8], out_len: usize, dll: &str, oodle: &mut Option<Oodle>) -> Result<Vec<u8>, String> {
    match codec {
        0 => Ok(comp.to_vec()), // stored
        4 => {
            if oodle.is_none() { *oodle = Some(Oodle::load(dll)?); }
            oodle.as_ref().unwrap().decompress(comp, out_len)
        }
        1 => {
            let mut out = Vec::with_capacity(out_len);
            flate2::read::ZlibDecoder::new(comp).read_to_end(&mut out).map_err(|x| x.to_string())?;
            Ok(out)
        }
        c => Err(format!("codec {c} not implemented (retail uses 0/4)")),
    }
}

/// Decode one entry's payload from the `.arc`. Three modes (verified against the full-file oracle):
///   * `comp_size == uncomp_size` (or codec 0) ⇒ stored raw, copy.
///   * `first_block_index == 0`                ⇒ ONE continuous stream, decode comp→uncomp in one call.
///   * `first_block_index != 0`                ⇒ PER-BLOCK: independently-compressed blocks from the
///     block table, decoded sequentially (skipping sentinels) until `uncomp_size` is reached.
pub fn decode_entry(arc: &mut File, t: &Tab, e: &Entry, dll: &str, oodle: &mut Option<Oodle>) -> Result<Vec<u8>, String> {
    arc.seek(SeekFrom::Start(e.offset as u64)).map_err(|x| x.to_string())?;
    if e.comp_size == e.uncomp_size || e.codec() == 0 {
        let mut buf = vec![0u8; e.comp_size as usize];
        arc.read_exact(&mut buf).map_err(|x| x.to_string())?;
        return Ok(buf);
    }
    let read_n = |arc: &mut File, n: usize| -> Result<Vec<u8>, String> {
        let mut b = vec![0u8; n];
        arc.read_exact(&mut b).map_err(|x| x.to_string())?;
        Ok(b)
    };
    if e.first_block() == 0 {
        let comp = read_n(arc, e.comp_size as usize)?;
        return decompress_codec(e.codec(), &comp, e.uncomp_size as usize, dll, oodle);
    }
    let mut out = Vec::with_capacity(e.uncomp_size as usize);
    let mut bi = e.first_block() as usize;
    while (out.len() as u32) < e.uncomp_size {
        let blk = t.blocks.get(bi).copied().ok_or(format!("block index {bi} oob"))?;
        if blk.is_sentinel() { bi += 1; continue; }
        let comp = read_n(arc, blk.comp_size as usize)?;
        out.extend_from_slice(&decompress_codec(e.codec(), &comp, blk.uncomp_size as usize, dll, oodle)?);
        bi += 1;
    }
    if out.len() != e.uncomp_size as usize {
        return Err(format!("per-block size {} != {}", out.len(), e.uncomp_size));
    }
    Ok(out)
}
