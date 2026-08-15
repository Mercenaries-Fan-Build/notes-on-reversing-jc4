//! SARC v3 — Avalanche "small archive": groups a MODEL's / ENTITY's files (real paths preserved),
//! nested inside a single TAB/ARC entry. Cracked from real samples (2026-08-15). See
//! ../../docs/formats/composite_assets.md.
//!
//! Layout (little-endian): header {u32 magic_len=4, "SARC", u32 version(3), u32 toc_size} — the TOC
//! spans 0x10..0x10+toc_size and member data begins at `toc_end`. Then u32 name_heap_size @0x10, the
//! name heap (NUL-terminated paths) from 0x14, then the entry table (one 20-byte record per named
//! file, to toc_end):  { u32 data_offset, u32 data_size, u32 name_hash, u32 ext_hash, u32 name_offset }.
//! An entry is **stored** when data_offset ≥ toc_end (its bytes are in this SARC) or an **external
//! reference** when data_offset == 0 (the file lives in another archive; only its name/size are here).
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
    /// Member bytes, or None for an external reference.
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
    let names_start = 0x14;
    let names_end = names_start + nheap;
    if toc_end > flen || names_end > toc_end {
        return Err("SARC: header sizes overrun".into());
    }
    // A well-formed 20-byte entry: name_offset in the heap, and data is either stored (≥ toc_end,
    // in-bounds) or an external reference (offset == 0).
    let is_entry = |p: usize| -> bool {
        if p + 20 > toc_end { return false; }
        let off = u32le(b, p) as usize;
        let sz = u32le(b, p + 4) as usize;
        let noff = u32le(b, p + 16) as usize;
        noff < nheap && (off == 0 || (off >= toc_end && off as u64 + sz as u64 <= flen as u64))
    };
    // Locate the table start: the position in the small window after the name heap whose run of
    // entries consumes the table (ends within one record of toc_end). Robust to alignment padding.
    let mut ts = None;
    let mut best = names_end;
    for start in names_end.saturating_sub(4)..=(names_end + 16).min(toc_end) {
        let mut p = start;
        while is_entry(p) { p += 20; }
        if p > best && toc_end - p < 20 { best = p; ts = Some(start); }
    }
    let ts = ts.ok_or("SARC: entry table not found")?;
    let mut out = Vec::new();
    let mut p = ts;
    while is_entry(p) {
        let off = u32le(b, p) as usize;
        out.push(SarcEntry {
            name: cstr(b, names_start + u32le(b, p + 16) as usize),
            name_hash: u32le(b, p + 8),
            ext_hash: u32le(b, p + 12),
            offset: off,
            size: u32le(b, p + 4) as usize,
            stored: off >= toc_end,
        });
        p += 20;
    }
    Ok(out)
}
