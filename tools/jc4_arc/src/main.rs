// jc4_arc — Just Cause 4 TAB/ARC v2 CLI. Thin wrapper over `jc4_formats`.
// See ../../docs/formats/tab_arc_v2.md and ../../docs/formats/name_hash.md.
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

use jc4_formats::hash::hashlittle;
use jc4_formats::oodle::Oodle;
use jc4_formats::tab::{decode_entry, default_oodle_dll, magic_ext, parse_tab};

fn cmd_header(tab: &str) {
    let b = std::fs::read(tab).unwrap();
    match parse_tab(&b) {
        Ok(t) => {
            let h = &t.header;
            println!("TAB v{}.{}  align=0x{:x}  block_uncomp=0x{:x}  max_comp_block=0x{:x}",
                h.version_major, h.version_minor, h.alignment, h.block_uncompressed_size, h.max_comp_block_size);
            println!("block_count={}  entries@0x{:x}  count={}  trailing={}",
                h.block_count, t.entries_off, t.entries.len(), t.trailing);
        }
        Err(e) => eprintln!("{e}: {tab}"),
    }
}

fn cmd_verify(tab: &str, arc: Option<&str>) {
    let b = std::fs::read(tab).unwrap();
    let t = match parse_tab(&b) { Ok(t) => t, Err(e) => { eprintln!("{e}"); return; } };
    let arc_len = arc.map(|p| File::open(p).and_then(|mut f| f.seek(SeekFrom::End(0))).unwrap_or(0));
    let align = t.header.alignment;
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
        t.entries.len(), if t.trailing == 0 { "YES" } else { "NO" },
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

fn load_filelist_map(path: &str) -> HashMap<u32, String> {
    let mut m = HashMap::new();
    if let Ok(txt) = std::fs::read_to_string(path) {
        for line in txt.lines() {
            let p = line.trim();
            if p.is_empty() || p.starts_with(';') { continue; }
            m.insert(hashlittle(p.as_bytes(), 0), p.to_string());
        }
    }
    m
}

// Extract payloads. With `names`/`match_sub`, only entries whose resolved path contains `match_sub`
// are decoded, and outputs are named by their real path (basename) instead of the hash — so
// `--filelist F --match .resourcebundle` (or `--match /models/`) pulls exactly a composite asset set.
fn cmd_extract(tab_path: &str, arc_path: &str, outdir: &str, limit: usize, dll: &str,
               names: Option<&HashMap<u32, String>>, match_sub: Option<&str>) {
    let b = std::fs::read(tab_path).unwrap();
    let t = match parse_tab(&b) { Ok(t) => t, Err(e) => { eprintln!("{e}"); return; } };
    let mut arc = File::open(arc_path).unwrap();
    std::fs::create_dir_all(outdir).unwrap();
    let mut oodle: Option<Oodle> = None;
    let (mut ok, mut fail) = (0u32, 0u32);
    let sub = match_sub.map(|s| s.to_lowercase());
    for e in t.entries.iter() {
        if ok as usize >= limit { break; }
        let path = names.and_then(|m| m.get(&e.name_hash));
        if let Some(s) = &sub {
            match path { Some(p) if p.to_lowercase().contains(s.as_str()) => {}, _ => continue }
        }
        match decode_entry(&mut arc, &t, e, dll, &mut oodle) {
            Ok(data) => {
                let out = match path {
                    Some(p) => format!("{outdir}/{}", p.rsplit(['/', '\\']).next().unwrap_or(p)),
                    None => format!("{outdir}/{:08x}.{}", e.name_hash, magic_ext(&data)),
                };
                if let Err(x) = File::create(&out).and_then(|mut f| f.write_all(&data)) {
                    eprintln!("write {out}: {x}"); fail += 1;
                } else { ok += 1; }
            }
            Err(x) => { eprintln!("entry {:08x} (codec {}): {x}", e.name_hash, e.codec()); fail += 1; }
        }
    }
    eprintln!("extracted {ok} files to {outdir}  ({fail} failed)");
}

// Map a gibbed-style filelist (one resource path per line, ';' comments) onto a .tab via hashlittle.
fn cmd_names(tab: &str, filelist: &str) {
    let b = std::fs::read(tab).unwrap();
    let t = match parse_tab(&b) { Ok(t) => t, Err(e) => { eprintln!("{e}"); return; } };
    let mut map: HashMap<u32, String> = HashMap::new();
    let (mut lines, mut collisions) = (0u32, 0u32);
    for line in std::fs::read_to_string(filelist).unwrap().lines() {
        let p = line.trim();
        if p.is_empty() || p.starts_with(';') { continue; }
        lines += 1;
        if let Some(prev) = map.insert(hashlittle(p.as_bytes(), 0), p.to_string()) {
            if prev != p { collisions += 1; }
        }
    }
    let entry_hashes: HashSet<u32> = t.entries.iter().map(|e| e.name_hash).collect();
    let named = entry_hashes.iter().filter(|h| map.contains_key(h)).count();
    let hit = map.keys().filter(|h| entry_hashes.contains(h)).count();
    println!("tab entries: {}  distinct hashes: {}", t.entries.len(), entry_hashes.len());
    println!("filelist paths: {lines}  (hash collisions: {collisions})");
    println!("entries NAMED by filelist: {named}/{} ({:.0}%)", entry_hashes.len(),
        100.0 * named as f64 / entry_hashes.len().max(1) as f64);
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
        eprintln!("  jc4_arc header <tab>                            header + block table + entry count");
        eprintln!("  jc4_arc verify <tab> [arc]                      hard oracle (consume/aligned/monotonic/in-bounds)");
        eprintln!("  jc4_arc list   <tab> [n]                        list first n entries");
        eprintln!("  jc4_arc hex    <tab> [nbytes]                   hexdump raw bytes");
        eprintln!("  jc4_arc hash   <string>                         lookup3 hashlittle(0) name hash");
        eprintln!("  jc4_arc names  <tab> <filelist>                 un-hash entries via a filelist");
        eprintln!("  jc4_arc extract <tab> <arc> <outdir> [limit]    decode payloads (raw/zlib/Oodle)");
        return;
    }
    match a[1].as_str() {
        "header" => cmd_header(&a[2]),
        "verify" => cmd_verify(&a[2], a.get(3).map(|s| s.as_str())),
        "list" => cmd_list(&a[2], a.get(3).and_then(|s| s.parse().ok()).unwrap_or(32)),
        "hex" => cmd_hex(&a[2], a.get(3).and_then(|s| s.parse().ok()).unwrap_or(256)),
        "hash" => println!("{:08x}  {:?}", hashlittle(a[2].as_bytes(), 0), a[2]),
        "names" => {
            if a.len() < 4 { eprintln!("usage: jc4_arc names <tab> <filelist>"); return; }
            cmd_names(&a[2], &a[3]);
        }
        "extract" => {
            if a.len() < 5 {
                eprintln!("usage: jc4_arc extract <tab> <arc> <outdir> [limit] [--filelist F --match SUB]");
                return;
            }
            let limit = a.get(5).filter(|s| !s.starts_with("--")).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
            let fl = a.iter().position(|s| s == "--filelist").and_then(|i| a.get(i + 1)).map(|p| load_filelist_map(p));
            let msub = a.iter().position(|s| s == "--match").and_then(|i| a.get(i + 1)).cloned();
            cmd_extract(&a[2], &a[3], &a[4], limit, &default_oodle_dll(), fl.as_ref(), msub.as_deref());
        }
        other => eprintln!("unknown command {other}"),
    }
}
