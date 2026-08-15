//! SARC v3 — Avalanche "small archive": the container that groups a MODEL's or ENTITY's files (with
//! real paths preserved), nested inside a single TAB/ARC entry. Cracked from real samples (2026-08-15).
//! See ../../docs/formats/composite_assets.md.
//!
//! Layout (little-endian):
//!   0x00 u32 magic_len = 4
//!   0x04 char[4] "SARC"
//!   0x08 u32 version (3)
//!   0x0C u32 toc_size            (TOC spans 0x10 .. 0x10+toc_size; member data begins at toc_end)
//!   0x10 u32 name_heap_size      (name heap = NUL-terminated paths, from 0x14)
//!   ...  name heap ...
//!   ...  entry table (20 B each): { u32 data_offset, u32 data_size, u32 name_hash, u32 ext_hash,
//!                                   u32 name_offset (into the name heap) }
//!        — the first entry's data_offset == toc_end (anchor); a trailing sub-table/padding may follow
//!          the entries before toc_end.
fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }
fn cstr(b: &[u8], o: usize) -> String {
    let end = b[o..].iter().position(|&c| c == 0).map(|p| o + p).unwrap_or(b.len());
    String::from_utf8_lossy(&b[o..end]).into_owned()
}

pub struct SarcEntry {
    pub name: String,   // resource path (from the name heap)
    pub name_hash: u32,
    pub ext_hash: u32,  // hash of the file type/extension
    pub offset: usize,  // member data offset within the SARC
    pub size: usize,
}
impl SarcEntry {
    pub fn data<'a>(&self, sarc: &'a [u8]) -> &'a [u8] { &sarc[self.offset..self.offset + self.size] }
}

pub fn parse(b: &[u8]) -> Result<Vec<SarcEntry>, String> {
    if b.len() < 0x14 || u32le(b, 0) != 4 || &b[4..8] != b"SARC" {
        return Err("not a SARC".into());
    }
    let flen = b.len();
    let toc_end = 0x10 + u32le(b, 0x0C) as usize;
    let nheap = u32le(b, 0x10) as usize;
    let names_start = 0x14;
    let names_end = names_start + nheap;
    if toc_end > flen || names_end > flen {
        return Err("SARC: header sizes overrun file".into());
    }
    // Locate the entry table: the first position at/after the name heap whose 20-byte record is a
    // valid entry (data in [toc_end, EOF], name_offset within the heap). Skips any alignment padding.
    let valid = |p: usize| -> bool {
        if p + 20 > toc_end { return false; }
        let off = u32le(b, p) as usize;
        let sz = u32le(b, p + 4) as usize;
        let noff = u32le(b, p + 16) as usize;
        off >= toc_end && off + sz <= flen && noff < nheap
    };
    let ts = (names_end..(names_end + 16).min(toc_end)).find(|&p| valid(p))
        .ok_or("SARC: entry table not found")?;
    let mut out = Vec::new();
    let mut p = ts;
    while valid(p) {
        out.push(SarcEntry {
            offset: u32le(b, p) as usize,
            size: u32le(b, p + 4) as usize,
            name_hash: u32le(b, p + 8),
            ext_hash: u32le(b, p + 12),
            name: cstr(b, names_start + u32le(b, p + 16) as usize),
        });
        p += 20;
    }
    Ok(out)
}
