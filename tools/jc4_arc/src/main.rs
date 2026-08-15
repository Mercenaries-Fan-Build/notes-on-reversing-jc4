// jc4_arc — Just Cause 4 Avalanche TAB/ARC v2 reader.
//
// TAB v2 format CRACKED (double-blind, 2026-08-15; both investigators converged, exact-consume on 3
// files). Decomp oracle: the v2 parser is FUN_140f92be0; codec dispatch FUN_14ad55350; entries are
// hash-sorted for binary search (FUN_140f92890). See ../../docs/formats/tab_arc_v2.md.
//
// Layout:
//   Header (24 bytes):
//     0x00 char[4] magic "TAB\0"
//     0x04 u16     version_major = 2
//     0x06 u16     version_minor = 1
//     0x08 u32     alignment = 0x1000
//     0x0C u32     reserved = 0
//     0x10 u32     max_compressed_block_size (scratch hint; 0 when no real blocks)  [open: exact use]
//     0x14 u32     block_uncompressed_size = 0x80000 (512 KB) (0 when no real blocks)
//   0x18 u32 block_count
//   0x1C Block[block_count]      8 bytes each: { u32 compressed_size; u32 uncompressed_size }
//                               (0xFFFFFFFF,0xFFFFFFFF) records are sentinels/separators
//   entries (20 bytes each, to EOF): { u32 name_hash; u32 offset; u32 compressed_size;
//                                      u32 uncompressed_size; u32 flags }
//     flags: bits[15:0]=first_block_index, bits[23:16]=codec_id, bit[24]=multi_block
//     codec_id: 0=none(raw) 1=zlib 2=lz4 3=zstd 4=oodle   (retail arcs use only 0 and 4)
//   offset is a physical .arc byte offset, multiple of alignment, monotonic non-decreasing.
//
// Oodle/zstd/zlib extraction of payloads is a follow-up (needs the game's oo2core_7_win64.dll for
// codec 4). This tool parses + verifies the ToC and lists/inspects entries.

mod oodle;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

const GAME_DIR: &str = "C:/Program Files (x86)/Steam/steamapps/common/Just Cause 4";

// Bob Jenkins lookup3 hashlittle — the JC4 engine string hash (initval 0, string as-is, no NUL).
// PROVEN: reproduces all 12,130 (string→hash) pairs from ADF string-hash tables + instance names.
// This is the hash behind ADF type/name/string-hashes and (path-keyed) TAB entry name_hashes.
mod hashlittle {
    fn rot(x: u32, k: u32) -> u32 { x.rotate_left(k) }
    pub fn hash(data: &[u8], initval: u32) -> u32 {
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
}

fn u16le(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }

const HDR_LEN: usize = 0x18; // 24-byte fixed header

struct Header {
    version_major: u16,
    version_minor: u16,
    alignment: u32,
    max_comp_block_size: u32,
    block_uncompressed_size: u32,
    block_count: u32,
}

#[derive(Clone, Copy)]
struct Entry {
    name_hash: u32,
    offset: u32,
    comp_size: u32,
    uncomp_size: u32,
    flags: u32,
}

impl Entry {
    fn first_block(&self) -> u16 { (self.flags & 0xFFFF) as u16 }
    fn codec(&self) -> u8 { ((self.flags >> 16) & 0xFF) as u8 }
    fn multi_block(&self) -> bool { (self.flags >> 24) & 1 == 1 }
    fn codec_name(&self) -> &'static str {
        match self.codec() {
            0 => "raw", 1 => "zlib", 2 => "lz4", 3 => "zstd", 4 => "oodle", _ => "?",
        }
    }
}

#[derive(Clone, Copy)]
struct Block {
    comp_size: u32,
    uncomp_size: u32,
}
impl Block {
    fn is_sentinel(&self) -> bool { self.comp_size == 0xFFFFFFFF && self.uncomp_size == 0xFFFFFFFF }
}

struct Tab {
    header: Header,
    blocks: Vec<Block>,
    entries_off: usize,
    entries: Vec<Entry>,
    trailing: usize, // bytes left over after the entry table (must be 0)
}

fn parse_tab(b: &[u8]) -> Result<Tab, String> {
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

fn cmd_header(tab: &str) {
    let b = std::fs::read(tab).unwrap();
    match parse_tab(&b) {
        Ok(t) => {
            let h = &t.header;
            println!("TAB v{}.{}  align=0x{:x}  block_uncomp=0x{:x}  max_comp_block=0x{:x}",
                h.version_major, h.version_minor, h.alignment, h.block_uncompressed_size, h.max_comp_block_size);
            println!("block_count={}  block_table=0x1C..0x{:x}  entries@0x{:x}  count={}  trailing={}",
                h.block_count, t.entries_off, t.entries_off, t.entries.len(), t.trailing);
        }
        Err(e) => eprintln!("{e}: {tab}"),
    }
}

// Verify the ToC against the hard oracle, optionally against the sibling .arc.
fn cmd_verify(tab: &str, arc: Option<&str>) {
    let b = std::fs::read(tab).unwrap();
    let t = match parse_tab(&b) { Ok(t) => t, Err(e) => { eprintln!("{e}"); return; } };
    let arc_len = arc.map(|p| File::open(p).and_then(|mut f| f.seek(SeekFrom::End(0))).unwrap_or(0));
    let align = t.header.alignment as u32;
    let (mut aligned, mut monotonic, mut in_bounds) = (0u32, 0u32, 0u32);
    let mut last: u32 = 0;
    for e in &t.entries {
        if align != 0 && e.offset % align == 0 { aligned += 1; }
        if e.offset >= last { monotonic += 1; }
        if arc_len.map_or(true, |al| e.offset as u64 + e.comp_size as u64 <= al) { in_bounds += 1; }
        last = e.offset;
    }
    let n = t.entries.len().max(1) as f64;
    println!("{tab}");
    println!("  entries={}  exact-consume={}  aligned={:.0}%  monotonic={:.0}%  in-bounds={:.0}%{}",
        t.entries.len(),
        if t.trailing == 0 { "YES" } else { "NO" },
        100.0 * aligned as f64 / n, 100.0 * monotonic as f64 / n, 100.0 * in_bounds as f64 / n,
        arc_len.map_or(String::new(), |al| format!("  (arc_len={al})")));
    let pass = t.trailing == 0
        && aligned == t.entries.len() as u32
        && monotonic == t.entries.len() as u32
        && arc_len.map_or(true, |_| in_bounds == t.entries.len() as u32);
    println!("  ORACLE: {}", if pass { "PASS" } else { "FAIL" });
}

fn cmd_list(tab: &str, limit: usize) {
    let b = std::fs::read(tab).unwrap();
    let t = match parse_tab(&b) { Ok(t) => t, Err(e) => { eprintln!("{e}"); return; } };
    println!("# {} entries", t.entries.len());
    println!("{:>10}  {:>10}  {:>10}  {:>10}  {:>10}  codec  blk  multi", "name_hash", "offset", "comp", "uncomp", "flags");
    for e in t.entries.iter().take(limit) {
        println!("{:08x}  {:>#10x}  {:>10}  {:>10}  {:08x}  {:>5}  {:>3}  {}",
            e.name_hash, e.offset, e.comp_size, e.uncomp_size, e.flags,
            e.codec_name(), e.first_block(), if e.multi_block() { "Y" } else { "" });
    }
}

fn magic_ext(b: &[u8]) -> &'static str {
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

// Decode one entry's payload from the .arc. Empirically established (ctypes probe + full-file oracle):
//   * comp_size == uncomp_size            ⇒ stored raw, copy.
//   * first_block_index (flags[15:0]) == 0 ⇒ ONE continuous Oodle stream, decode comp→uncomp in one call.
//   * first_block_index != 0               ⇒ PER-BLOCK: the payload is a run of independently-Oodle-
//                                            compressed blocks; decode block[i] (comp→uncomp) from the
//                                            block table, advancing sequentially, skipping sentinels,
//                                            until the entry's uncomp_size is reached.
// The two modes coexist in the same arc (continuous single-chunk files vs. per-block streamed files);
// first_block is the discriminator. Verified: single-shot on 03291a6f returns 0 but per-block yields it
// exactly; per-block fully reconstructs the 44.7 MB (2e70a6bc, blk 1..86) and 22 MB (4800f677, 173..215).
// Decompress one buffer by codec id (0=raw/None, 1=zlib, 4=Oodle) — matches gibbed's
// GetDecompress(CompressionType), so per-block entries dispatch on codec instead of assuming Oodle.
fn decompress_codec(codec: u8, comp: &[u8], out_len: usize, dll: &str, oodle: &mut Option<oodle::Oodle>) -> Result<Vec<u8>, String> {
    match codec {
        0 => Ok(comp.to_vec()), // stored
        4 => {
            if oodle.is_none() { *oodle = Some(oodle::Oodle::load(dll)?); }
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

fn decode_entry(arc: &mut File, t: &Tab, e: &Entry, dll: &str, oodle: &mut Option<oodle::Oodle>) -> Result<Vec<u8>, String> {
    arc.seek(SeekFrom::Start(e.offset as u64)).map_err(|x| x.to_string())?;
    // fast path: whole-entry stored (comp==uncomp, or codec None — matches gibbed's raw-copy trigger)
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
        // one continuous stream (first_block==0 ⇒ block 0 is a no_block sentinel ⇒ single stream)
        let comp = read_n(arc, e.comp_size as usize)?;
        return decompress_codec(e.codec(), &comp, e.uncomp_size as usize, dll, oodle);
    }
    // per-block stream: each block independently compressed with the entry's codec
    let mut out = Vec::with_capacity(e.uncomp_size as usize);
    let mut bi = e.first_block() as usize;
    while (out.len() as u32) < e.uncomp_size {
        let blk = t.blocks.get(bi).copied().ok_or(format!("block index {bi} oob"))?;
        if blk.is_sentinel() { bi += 1; continue; } // separator, no arc payload
        let comp = read_n(arc, blk.comp_size as usize)?;
        out.extend_from_slice(&decompress_codec(e.codec(), &comp, blk.uncomp_size as usize, dll, oodle)?);
        bi += 1;
    }
    if out.len() != e.uncomp_size as usize {
        return Err(format!("per-block size {} != {}", out.len(), e.uncomp_size));
    }
    Ok(out)
}

fn cmd_extract(tab_path: &str, arc_path: &str, outdir: &str, limit: usize, dll: &str) {
    let b = std::fs::read(tab_path).unwrap();
    let t = match parse_tab(&b) { Ok(t) => t, Err(e) => { eprintln!("{e}"); return; } };
    let mut arc = File::open(arc_path).unwrap();
    std::fs::create_dir_all(outdir).unwrap();
    let mut oodle: Option<oodle::Oodle> = None;
    let (mut ok, mut fail) = (0u32, 0u32);
    for e in t.entries.iter().take(limit) {
        match decode_entry(&mut arc, &t, e, dll, &mut oodle) {
            Ok(data) => {
                let ext = magic_ext(&data);
                let path = format!("{outdir}/{:08x}.{ext}", e.name_hash);
                if let Err(x) = File::create(&path).and_then(|mut f| f.write_all(&data)) {
                    eprintln!("write {path}: {x}"); fail += 1;
                } else { ok += 1; }
            }
            Err(x) => { eprintln!("entry {:08x} (codec {} {}): {x}", e.name_hash, e.codec(),
                if e.multi_block() { "multi" } else { "single" }); fail += 1; }
        }
    }
    eprintln!("extracted {ok} files to {outdir}  ({fail} failed)");
}

// Map a gibbed-style filelist (one resource path per line, ';' comments) onto a .tab via hashlittle,
// reporting coverage. Doubles as an at-scale validation of our hash: if it's correct, named-count should
// match the filelist's own stated coverage.
fn cmd_names(tab: &str, filelist: &str) {
    let b = std::fs::read(tab).unwrap();
    let t = match parse_tab(&b) { Ok(t) => t, Err(e) => { eprintln!("{e}"); return; } };
    let mut map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    let (mut lines, mut collisions) = (0u32, 0u32);
    for line in std::fs::read_to_string(filelist).unwrap().lines() {
        let p = line.trim();
        if p.is_empty() || p.starts_with(';') { continue; }
        lines += 1;
        let h = hashlittle::hash(p.as_bytes(), 0);
        if let Some(prev) = map.insert(h, p.to_string()) {
            if prev != p { collisions += 1; }
        }
    }
    let entry_hashes: std::collections::HashSet<u32> = t.entries.iter().map(|e| e.name_hash).collect();
    let named = entry_hashes.iter().filter(|h| map.contains_key(h)).count();
    // how many filelist paths actually hit a real entry (validates the hash direction)
    let hit = map.keys().filter(|h| entry_hashes.contains(h)).count();
    println!("tab entries: {}  distinct hashes: {}", t.entries.len(), entry_hashes.len());
    println!("filelist paths: {lines}  (hash collisions: {collisions})");
    println!("entries NAMED by filelist: {named}/{} ({:.0}%)", entry_hashes.len(),
        100.0 * named as f64 / entry_hashes.len() as f64);
    println!("filelist paths that hit a real entry: {hit}");
    for e in t.entries.iter().take(12) {
        println!("  {:08x}  {}", e.name_hash, map.get(&e.name_hash).map(|s| s.as_str()).unwrap_or("<unknown>"));
    }
}

fn cmd_hex(tab: &str, n: usize) {
    let b = std::fs::read(tab).unwrap();
    let region = &b[0..n.min(b.len())];
    for (i, row) in region.chunks(16).enumerate() {
        let hex: String = row.iter().map(|x| format!("{:02x} ", x)).collect();
        let asc: String = row.iter().map(|&x| if (0x20..0x7f).contains(&x) { x as char } else { '.' }).collect();
        println!("{:08x}  {:<48}{}", i * 16, hex, asc);
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("jc4_arc — JC4 TAB/ARC v2 reader");
        eprintln!("usage:");
        eprintln!("  jc4_arc header <tab>            parse & print header + block table + entry count");
        eprintln!("  jc4_arc verify <tab> [arc]      check the hard oracle (exact-consume/aligned/monotonic/in-bounds)");
        eprintln!("  jc4_arc list   <tab> [n]        list first n entries (default 32)");
        eprintln!("  jc4_arc hex    <tab> [nbytes]   hexdump raw bytes (default 256)");
        eprintln!("  jc4_arc hash   <string>         lookup3 hashlittle(0) — the engine name hash");
        eprintln!("  jc4_arc extract <tab> <arc> <outdir> [limit]   decode payloads (raw/Oodle) -> outdir");
        eprintln!("      (Oodle DLL: {GAME_DIR}/oo2core_7_win64.dll; override via $JC4_OODLE_DLL)");
        return;
    }
    match a[1].as_str() {
        "header" => cmd_header(&a[2]),
        "verify" => cmd_verify(&a[2], a.get(3).map(|s| s.as_str())),
        "list" => cmd_list(&a[2], a.get(3).and_then(|s| s.parse().ok()).unwrap_or(32)),
        "hex" => cmd_hex(&a[2], a.get(3).and_then(|s| s.parse().ok()).unwrap_or(256)),
        "hash" => println!("{:08x}  {:?}", hashlittle::hash(a[2].as_bytes(), 0), a[2]),
        "names" => {
            if a.len() < 4 { eprintln!("usage: jc4_arc names <tab> <filelist>"); return; }
            cmd_names(&a[2], &a[3]);
        }
        "extract" => {
            if a.len() < 5 { eprintln!("usage: jc4_arc extract <tab> <arc> <outdir> [limit]"); return; }
            let limit = a.get(5).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
            let dll = std::env::var("JC4_OODLE_DLL")
                .unwrap_or_else(|_| format!("{GAME_DIR}/oo2core_7_win64.dll"));
            cmd_extract(&a[2], &a[3], &a[4], limit, &dll);
        }
        other => eprintln!("unknown command {other}"),
    }
}
