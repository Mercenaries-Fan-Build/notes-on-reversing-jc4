// jc4_arc — Just Cause 4 Avalanche TAB/ARC v2 reader. WIP.
//
// Forked in spirit from the Mercs 2 repo's `tools/jc2/jc2_arc`, but JC4 is a LATER Apex revision:
// its TAB is v2, not JC2's v1, so the entry table decode is DIFFERENT and is not yet verified.
//
// PROVEN so far (from the retail install survey, boot/game0.tab @ offset 0):
//   00: 54 41 42 00                "TAB\0"          magic
//   04: 02 00 01 00                version 2.1      (major=2, minor=1)
//   08: 00 10 00 00                alignment 0x1000 (arc entries are 0x1000-aligned)
//   0C: 00 00 00 00                (zero / reserved)
//   10: <entry table begins>
//
// UNVERIFIED — the entry struct. It is NOT JC2's 12-byte {hash,offset,size}: the raw bytes at 0x10
// carry interleaved 0xFFFFFFFF fields that break a 12-byte stride on the very first entry:
//   10: 2b 8c 07 00 | 00 00 08 00 | 0e 05 00 00 | ff ff ff ff
//   20: ff ff ff ff | b5 b6 00 00 | 00 00 08 00 | 60 d3 00 00
// Cracking the stride is the first double-blind task. This binary helps: `probe` scores candidate
// strides against a hard oracle (offsets 0x1000-aligned, monotonic, in-bounds of the .arc). Let the
// evidence pick the layout; do NOT hardcode a guess. See ../../docs/lineage_and_divergence.md.

use std::fs::File;
use std::io::{Seek, SeekFrom};

fn u16le(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }

const HDR_LEN: usize = 0x10;

struct Header {
    version_major: u16,
    version_minor: u16,
    alignment: u32,
}

fn read_header(b: &[u8]) -> Option<Header> {
    if b.len() < HDR_LEN || &b[0..4] != b"TAB\0" {
        return None;
    }
    Some(Header {
        version_major: u16le(b, 4),
        version_minor: u16le(b, 6),
        alignment: u32le(b, 8),
    })
}

fn cmd_header(tab: &str) {
    let b = std::fs::read(tab).unwrap();
    match read_header(&b) {
        Some(h) => {
            println!("TAB v{}.{}  alignment=0x{:x}  file={} bytes", h.version_major, h.version_minor, h.alignment, b.len());
            println!("entry region: {} bytes after 0x{:x} header", b.len() - HDR_LEN, HDR_LEN);
        }
        None => eprintln!("not a TAB file (bad magic): {}", tab),
    }
}

// Hexdump the entry region so a human/agent can eyeball structure.
fn cmd_hex(tab: &str, n: usize) {
    let b = std::fs::read(tab).unwrap();
    let region = &b[HDR_LEN.min(b.len())..(HDR_LEN + n).min(b.len())];
    for (i, row) in region.chunks(16).enumerate() {
        let addr = HDR_LEN + i * 16;
        let hex: String = row.iter().map(|x| format!("{:02x} ", x)).collect();
        let asc: String = row.iter().map(|&x| if (0x20..0x7f).contains(&x) { x as char } else { '.' }).collect();
        println!("{:08x}  {:<48}{}", addr, hex, asc);
    }
}

// Score a candidate entry layout: interpret entries as [hash u32 @0, offset u32 @off_field, size u32 @size_field]
// with a given total `stride`. Report what fraction of decoded offsets pass the oracle.
fn score_stride(b: &[u8], arc_len: Option<u64>, stride: usize, off_field: usize, size_field: usize, align: u32) {
    let region = b.len() - HDR_LEN;
    let n = region / stride;
    if n == 0 {
        println!("  stride {:>2} off@{} size@{}: no entries", stride, off_field, size_field);
        return;
    }
    let (mut aligned, mut monotonic, mut in_bounds) = (0u32, 0u32, 0u32);
    let mut last_off: u64 = 0;
    let mut first = Vec::new();
    for i in 0..n {
        let base = HDR_LEN + i * stride;
        if base + size_field + 4 > b.len() { break; }
        let off = u32le(b, base + off_field) as u64;
        let size = u32le(b, base + size_field) as u64;
        let hash = u32le(b, base);
        if off % align as u64 == 0 { aligned += 1; }
        if off >= last_off { monotonic += 1; }
        if arc_len.map_or(true, |al| off.saturating_add(size) <= al) { in_bounds += 1; }
        last_off = off;
        if i < 4 { first.push((hash, off, size)); }
    }
    let pct = |x: u32| 100.0 * x as f64 / n as f64;
    println!(
        "  stride {:>2} off@{:>2} size@{:>2}: n={:<6} aligned={:>3.0}% monotonic={:>3.0}% in-bounds={:>3.0}%",
        stride, off_field, size_field, n, pct(aligned), pct(monotonic), pct(in_bounds)
    );
    for (h, o, s) in first {
        println!("      e: hash={:08x} off={:#x} size={}", h, o, s);
    }
}

// Try a spread of plausible layouts. The right one should hit ~100% aligned + monotonic + in-bounds.
fn cmd_probe(tab: &str, arc: Option<&str>) {
    let b = std::fs::read(tab).unwrap();
    let h = match read_header(&b) {
        Some(h) => h,
        None => { eprintln!("not a TAB file"); return; }
    };
    let arc_len = arc.map(|p| File::open(p).and_then(|mut f| f.seek(SeekFrom::End(0))).unwrap_or(0));
    println!("TAB v{}.{} align=0x{:x}; entry region {} bytes. arc_len={:?}",
        h.version_major, h.version_minor, h.alignment, b.len() - HDR_LEN, arc_len);
    println!("hypothesis sweep (want ~100% on all three):");
    // (stride, offset-field, size-field) candidates
    let candidates = [
        (12usize, 4usize, 8usize),   // JC2 v1 layout (expected to FAIL on JC4)
        (16, 4, 8),
        (16, 8, 12),
        (20, 4, 8),
        (20, 8, 12),
        (24, 4, 8),
        (24, 8, 16),
    ];
    for (stride, offf, sizef) in candidates {
        score_stride(&b, arc_len, stride, offf, sizef, h.alignment);
    }
    println!("\nnote: none of these is confirmed. Use `hex` + the decomp (TAB loader in JustCause4.exe)");
    println!("to derive the real struct, then replace this probe with a verified reader + round-trip test.");
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("jc4_arc — JC4 TAB/ARC v2 reader (WIP)");
        eprintln!("usage:");
        eprintln!("  jc4_arc header <tab>              parse & print the (proven) v2 header");
        eprintln!("  jc4_arc hex    <tab> [nbytes]     hexdump the entry region (default 256)");
        eprintln!("  jc4_arc probe  <tab> [arc]        score candidate entry strides against the oracle");
        return;
    }
    match a[1].as_str() {
        "header" => cmd_header(&a[2]),
        "hex" => cmd_hex(&a[2], a.get(3).and_then(|s| s.parse().ok()).unwrap_or(256)),
        "probe" => cmd_probe(&a[2], a.get(3).map(|s| s.as_str())),
        other => eprintln!("unknown command {other}"),
    }
}
