//! Bob Jenkins lookup3 `hashlittle` — the JC4 engine string hash (use `initval = 0`).
//!
//! PROVEN: reproduces all 12,130 (string→hash) pairs from ADF string-hash tables + instance names,
//! and 2,161 gibbed-filelist resource-path → TAB-entry-hash matches. Byte-for-byte identical to DECA
//! `hash32_func`. TAB entries key the full lowercase forward-slash resource path (with extension, no
//! leading/trailing slash); ADF keys strings as-is. See ../../docs/formats/name_hash.md.

fn rot(x: u32, k: u32) -> u32 { x.rotate_left(k) }

/// lookup3 `hashlittle(data, initval)` → 32-bit hash. JC4 uses `initval = 0`.
pub fn hashlittle(data: &[u8], initval: u32) -> u32 {
    let mut len = data.len();
    let init = 0xdeadbeef_u32.wrapping_add(len as u32).wrapping_add(initval);
    let (mut a, mut b, mut c) = (init, init, init);
    let rd = |s: &[u8], o: usize| u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]);
    let mut off = 0;
    while len > 12 {
        a = a.wrapping_add(rd(data, off));
        b = b.wrapping_add(rd(data, off + 4));
        c = c.wrapping_add(rd(data, off + 8));
        a = a.wrapping_sub(c); a ^= rot(c, 4); c = c.wrapping_add(b);
        b = b.wrapping_sub(a); b ^= rot(a, 6); a = a.wrapping_add(c);
        c = c.wrapping_sub(b); c ^= rot(b, 8); b = b.wrapping_add(a);
        a = a.wrapping_sub(c); a ^= rot(c, 16); c = c.wrapping_add(b);
        b = b.wrapping_sub(a); b ^= rot(a, 19); a = a.wrapping_add(c);
        c = c.wrapping_sub(b); c ^= rot(b, 4); b = b.wrapping_add(a);
        off += 12; len -= 12;
    }
    let mut t = [0u8; 12];
    t[..len].copy_from_slice(&data[off..off + len]);
    a = a.wrapping_add(rd(&t, 0));
    b = b.wrapping_add(rd(&t, 4));
    c = c.wrapping_add(rd(&t, 8));
    if len == 0 { return c; }
    c ^= b; c = c.wrapping_sub(rot(b, 14));
    a ^= c; a = a.wrapping_sub(rot(c, 11));
    b ^= a; b = b.wrapping_sub(rot(a, 25));
    c ^= b; c = c.wrapping_sub(rot(b, 16));
    a ^= c; a = a.wrapping_sub(rot(c, 4));
    b ^= a; b = b.wrapping_sub(rot(a, 14));
    c ^= b; c = c.wrapping_sub(rot(b, 24));
    c
}

#[cfg(test)]
mod tests {
    use super::hashlittle;
    #[test]
    fn known_pairs() {
        assert_eq!(hashlittle(b"BloomContrast", 0), 0x2f6ea6e9);
        assert_eq!(hashlittle(b"EnvLightIntensityScale", 0), 0x694409f1);
        assert_eq!(hashlittle(b"environment/presets/aerial.environc", 0), 0x0ec6df48);
    }
}
