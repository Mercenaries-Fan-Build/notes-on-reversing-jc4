//! SARC v3 — Avalanche "small archive": groups a MODEL's / ENTITY's files (real paths preserved),
//! nested inside a single TAB/ARC entry. Cracked from real samples (2026-08-15); the entry layout was
//! pinned by requiring `name_hash == hashlittle(name)` on every record. See
//! ../../docs/formats/composite_assets.md.
//!
//! Layout (little-endian): header {u32 magic_len=4, "SARC", u32 version(3), u32 toc_size} — the TOC
//! spans 0x10..0x10+toc_size; member data begins at `toc_end`. Then u32 name_heap_size @0x10, the name
//! heap (NUL-terminated paths) from 0x14, then the entry table — one **20-byte** record per named file:
//!   { u32 ext_hash, u32 name_offset (into heap), u32 data_offset, u32 data_size, u32 name_hash }.
//! An entry is **stored** when data_offset ≥ toc_end (bytes present here) or an **external reference**
//! when data_offset == 0 (the file lives in another archive; only its name/size are recorded).
use crate::hash::hashlittle;

fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }
fn cstr(b: &[u8], o: usize) -> String {
    let end = b[o..].iter().position(|&c| c == 0).map(|p| o + p).unwrap_or(b.len());
    String::from_utf8_lossy(&b[o..end]).into_owned()
}

pub struct SarcEntry {
    pub name: String,   // resource path (from the name heap)
    pub name_hash: u32,
    pub ext_hash: u32,  // hash of the file type/extension
    pub offset: usize,  // member data offset within the SARC (0 if external reference)
    pub size: usize,
    pub stored: bool,   // true = bytes present in this SARC; false = external reference
}
impl SarcEntry {
    pub fn data<'a>(&self, sarc: &'a [u8]) -> Option<&'a [u8]> {
        if self.stored { Some(&sarc[self.offset..self.offset + self.size]) } else { None }
    }
}

pub fn parse(b: &[u8]) -> Result<Vec<SarcEntry>, String> {
    if b.len() < 0x14 || u32le(b, 0) != 4 || &b[4..8] != b"SARC" {
        return Err("not a SARC".into());
    }
    let flen = b.len();
    let toc_end = 0x10 + u32le(b, 0x0C) as usize;
    let nheap = u32le(b, 0x10) as usize;
    let ns = 0x14; // name heap start
    let names_end = ns + nheap;
    if toc_end > flen || names_end > toc_end {
        return Err("SARC: header sizes overrun".into());
    }
    // A 20-byte record is well-formed when: name_offset is in the heap, data is stored (≥ toc_end,
    // in-bounds) or an external reference (offset 0), AND name_hash == hashlittle(resolved name).
    let rec_ok = |p: usize| -> bool {
        if p + 20 > toc_end { return false; }
        let noff = u32le(b, p + 4) as usize;
        if noff >= nheap { return false; }
        let off = u32le(b, p + 8) as usize;
        let sz = u32le(b, p + 12) as usize;
        if !(off == 0 || (off >= toc_end && off as u64 + sz as u64 <= flen as u64)) { return false; }
        hashlittle(cstr(b, ns + noff).as_bytes(), 0) == u32le(b, p + 16)
    };
    // Locate the table start: the position in the small window around the name-heap end whose run of
    // valid records consumes the table (ends within one record of toc_end).
    let mut ts = None;
    let mut best = 0usize;
    for start in names_end.saturating_sub(8)..=(names_end + 12).min(toc_end) {
        let mut p = start;
        while rec_ok(p) { p += 20; }
        if p > best && toc_end - p < 20 { best = p; ts = Some(start); }
    }
    let ts = ts.ok_or("SARC: entry table not found (no hash-valid run)")?;
    let mut out = Vec::new();
    let mut p = ts;
    while rec_ok(p) {
        let off = u32le(b, p + 8) as usize;
        out.push(SarcEntry {
            ext_hash: u32le(b, p),
            name: cstr(b, ns + u32le(b, p + 4) as usize),
            offset: off,
            size: u32le(b, p + 12) as usize,
            name_hash: u32le(b, p + 16),
            stored: off >= toc_end,
        });
        p += 20;
    }
    Ok(out)
}
