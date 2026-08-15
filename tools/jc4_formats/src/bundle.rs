//! resourcebundle — a flat bundle of a world-node's typed resources. Each member is a
//! `{u32 name_hash, u32 type_hash, u32 size, data[size]}` record, tightly packed to EOF (no padding).
//! Members are almost always ADF (e.g. `SBreakableCollection`, `SCoverSectionCollection`). This is the
//! composition root of a "structure" / world-node — NOT a model bundle (models use SARC). See
//! ../../docs/formats/composite_assets.md.
fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }

pub struct Member {
    pub name_hash: u32,
    pub type_hash: u32,
    pub offset: usize, // start of this member's data within the bundle
    pub size: usize,
}

impl Member {
    pub fn data<'a>(&self, bundle: &'a [u8]) -> &'a [u8] { &bundle[self.offset..self.offset + self.size] }
}

/// Parse a resourcebundle into its member records. Stops cleanly at EOF; a trailing partial record
/// (shouldn't happen on retail bundles) ends parsing.
pub fn parse(b: &[u8]) -> Vec<Member> {
    let mut out = Vec::new();
    let mut p = 0usize;
    while p + 12 <= b.len() {
        let name_hash = u32le(b, p);
        let type_hash = u32le(b, p + 4);
        let size = u32le(b, p + 8) as usize;
        let data = p + 12;
        if size == 0 || data + size > b.len() { break; }
        out.push(Member { name_hash, type_hash, offset: data, size });
        p = data + size;
    }
    out
}
