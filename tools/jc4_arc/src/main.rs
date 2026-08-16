// jc4_arc — Just Cause 4 TAB/ARC v2 CLI. Thin wrapper over `jc4_formats`.
// See ../../docs/formats/tab_arc_v2.md and ../../docs/formats/name_hash.md.
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

use jc4_formats::hash::hashlittle;
use jc4_formats::oodle::Oodle;
use jc4_formats::tab::{decode_entry, oodle_dll_from_env, magic_ext, parse_tab};

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

// Read a resourcebundle (world-node/structure root): list members, and for ADF members summarize the
// composition — the root instance's fields with array counts (e.g. SBreakableCollection's
// model_instances / breakable_instances / effect_instances / navmesh_cutters). This IS the Structure
// inspector's data layer; it needs no new crack beyond ADF (already proven).
fn cmd_bundle(path: &str) {
    let b = std::fs::read(path).unwrap();
    let members = jc4_formats::bundle::parse(&b);
    println!("# resourcebundle: {} member(s), {} bytes", members.len(), b.len());
    for (i, m) in members.iter().enumerate() {
        let data = m.data(&b);
        let ext = magic_ext(data);
        println!("[{i}] name {:08x}  type {:08x}  {:>9} B  {}", m.name_hash, m.type_hash, m.size, ext);
        if ext != "adf" { continue; }
        let adf = match jc4_formats::adf::parse(data.to_vec()) { Ok(a) => a, Err(_) => continue };
        let v = adf.decode_instances();
        if let Some(root) = v.as_object() {
            for (name, val) in root {
                let obj = match val.as_object() { Some(o) => o, None => continue };
                println!("    {name}:  ({} fields)", obj.len());
                for (k, fv) in obj {
                    if let Some(arr) = fv.as_array() {
                        println!("      {k}: [{} items]", arr.len());
                    } else if let Some(o2) = fv.as_object() {
                        println!("      {k}: {{{} fields}}", o2.len());
                    } else {
                        let s = fv.to_string();
                        println!("      {k}: {}", if s.len() > 60 { &s[..60] } else { &s });
                    }
                }
            }
        }
    }
}

// List a SARC's members (a model's / entity's grouped files, with real paths).
fn cmd_sarc(path: &str, outdir: Option<&str>) {
    let b = std::fs::read(path).unwrap();
    match jc4_formats::sarc::parse(&b) {
        Ok(members) => {
            let stored = members.iter().filter(|m| m.stored).count();
            println!("# SARC: {} member(s) — {} stored, {} external ref(s), {} bytes",
                members.len(), stored, members.len() - stored, b.len());
            if let Some(dir) = outdir { std::fs::create_dir_all(dir).ok(); }
            for m in &members {
                match m.data(&b) {
                    Some(d) => {
                        println!("  {:>10} B  {:>5}  {}", m.size, magic_ext(d), m.name);
                        if let Some(dir) = outdir {
                            let out = format!("{dir}/{}", basename(&m.name));
                            if let Err(e) = File::create(&out).and_then(|mut f| f.write_all(d)) { eprintln!("  write {out}: {e}"); }
                        }
                    }
                    None => println!("  {:>10} B    ref  → {}", m.size, m.name),
                }
            }
        }
        Err(e) => eprintln!("{e}: {path}"),
    }
}

// Model inspector: walk a model SARC → decode each `.modelc` (Avalanche AMF) → resolve its `.meshc`
// member → print the assembly (LODs, materials + texture slots, mesh vertex/index stats). Uses only
// SARC + ADF (both proven) + AVTX names — no new crack. The workshop's Model inspector data layer.
fn basename(s: &str) -> &str { s.rsplit(['/', '\\']).next().unwrap_or(s) }

fn summarize_meshc(data: &[u8]) {
    let adf = match jc4_formats::adf::parse(data.to_vec()) { Ok(a) => a, Err(_) => return };
    let v = adf.decode_instances();
    // the header instance is the one carrying "LodGroups"
    let hdr = v.as_object().and_then(|o| o.values().find(|iv| iv.get("LodGroups").is_some()));
    let hdr = match hdr { Some(h) => h, None => return };
    let lods = hdr.get("LodGroups").and_then(|v| v.as_array());
    let (mut verts, mut idx, mut meshes) = (0u64, 0u64, 0u64);
    if let Some(groups) = lods {
        for g in groups {
            if let Some(ms) = g.get("Meshes").and_then(|v| v.as_array()) {
                for m in ms {
                    meshes += 1;
                    verts += m.get("VertexCount").and_then(|v| v.as_u64()).unwrap_or(0);
                    idx += m.get("IndexCount").and_then(|v| v.as_u64()).unwrap_or(0);
                }
            }
        }
    }
    println!("    mesh: {} LOD group(s), {} mesh(es), {} verts, {} indices",
        lods.map(|a| a.len()).unwrap_or(0), meshes, verts, idx);
    if let Some(hp) = hdr.get("HighLodPath").and_then(|v| v.as_str()) {
        if !hp.is_empty() { println!("    hi-res: {}", basename(hp)); }
    }
}

fn cmd_model(sarc_path: &str) {
    let b = std::fs::read(sarc_path).unwrap();
    let members = match jc4_formats::sarc::parse(&b) { Ok(m) => m, Err(e) => { eprintln!("{e}"); return; } };
    let by_name: HashMap<&str, usize> = members.iter().enumerate().map(|(i, m)| (m.name.as_str(), i)).collect();
    let modelcs: Vec<&jc4_formats::sarc::SarcEntry> =
        members.iter().filter(|m| m.stored && m.name.ends_with(".modelc")).collect();
    println!("# {} model part(s) in {}", modelcs.len(), basename(sarc_path));
    for mc in modelcs {
        let adf = match jc4_formats::adf::parse(mc.data(&b).unwrap().to_vec()) { Ok(a) => a, Err(_) => continue };
        let v = adf.decode_instances();
        let model = match v.as_object().and_then(|o| o.values().next()) { Some(m) => m, None => continue };
        println!("\n▸ {}", basename(&mc.name));
        let lods = model.get("LodSlots").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        let lf = model.get("LodFactor").and_then(|v| v.as_f64()).unwrap_or(0.0);
        println!("  {lods} LOD(s), LodFactor {lf}");
        if let Some(mats) = model.get("Materials").and_then(|v| v.as_array()) {
            for mat in mats {
                let name = mat.get("Name").and_then(|v| v.as_str()).unwrap_or("?");
                let rb = mat.get("RenderBlockId").and_then(|v| v.as_str()).unwrap_or("?");
                println!("  material '{name}'  [{rb}]");
                if let Some(texs) = mat.get("Textures").and_then(|v| v.as_array()) {
                    for t in texs.iter().filter_map(|v| v.as_str()).filter(|t| !t.is_empty() && !t.contains("dummies/")) {
                        println!("      tex: {}", basename(t));
                    }
                }
            }
        }
        if let Some(mp) = model.get("Mesh").and_then(|v| v.as_str()) {
            match by_name.get(mp).and_then(|&i| members[i].data(&b)) {
                Some(md) => summarize_meshc(md),
                None => println!("    mesh: {} (external)", basename(mp)),
            }
        }
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

// Scan a decoded blob for path-like ASCII strings and add them to `out`. This is how we un-hash:
// SARC name heaps, ADF/RTPC string fields and resourcebundle members all embed *real* asset paths as
// plain ASCII in the decompressed bytes. A run of path chars that contains a '/' and a '.' (extension
// in the basename) is a candidate; we normalise to lowercase/forward-slash (how TAB paths are hashed).
fn harvest_paths(data: &[u8], out: &mut std::collections::BTreeSet<String>) {
    let is_pc = |b: u8| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'/' | b'\\' | b'.' | b'-');
    let n = data.len();
    let mut i = 0;
    while i < n {
        if !is_pc(data[i]) { i += 1; continue; }
        let start = i;
        while i < n && is_pc(data[i]) { i += 1; }
        let s = &data[start..i];
        if s.len() < 6 || !s.contains(&b'/') { continue; }
        let st = match std::str::from_utf8(s) { Ok(x) => x, Err(_) => continue };
        let norm = st.to_ascii_lowercase().replace('\\', "/");
        // no doubled separators, and a real top-level directory (lowercase word: locations/ animations/ ui/ …)
        if norm.contains("//") || norm.contains("..") { continue; }
        let first = norm.split('/').next().unwrap_or("");
        if first.len() < 2 || !first.starts_with(|c: char| c.is_ascii_lowercase())
            || !first.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_') { continue; }
        // basename.ext, with a real-looking extension (letter-led, alphanumeric, ≤12)
        let base = norm.rsplit('/').next().unwrap_or("");
        let ext = match base.rsplit_once('.') { Some((stem, e)) if !stem.is_empty() => e, _ => continue };
        if ext.len() < 2 || ext.len() > 12
            || !ext.starts_with(|c: char| c.is_ascii_alphabetic())
            || !ext.bytes().all(|c| c.is_ascii_alphanumeric()) { continue; }
        // reject binary noise: require majority-alphanumeric AND a real word (≥3-letter run)
        let alnum = norm.bytes().filter(|c| c.is_ascii_alphanumeric()).count();
        if alnum * 2 < norm.len() { continue; }
        let mut run = 0u32;
        let has_word = norm.bytes().any(|c| { run = if c.is_ascii_alphabetic() { run + 1 } else { 0 }; run >= 3 });
        if !has_word { continue; }
        out.insert(norm);
    }
}

/// Walk `<game_dir>/archives_win64` for `.tab`/`.arc` pairs (recursively).
fn hmddsc_of(ddsc: &str) -> String { format!("{}.hmddsc", ddsc.trim_end_matches(".ddsc")) }

/// Fetch raw bytes for a set of resource paths: from stored SARC members, else (with a game dir) by
/// scanning the archives once for the path hashes.
fn fetch_paths(needed: &std::collections::BTreeSet<String>, sarc: &[u8], game_dir: Option<&str>, dll: &str)
    -> HashMap<String, Vec<u8>> {
    let members = jc4_formats::sarc::parse(sarc).unwrap_or_default();
    let mut out: HashMap<String, Vec<u8>> = HashMap::new();
    let mut external: HashMap<u32, String> = HashMap::new();
    for p in needed {
        match members.iter().find(|m| m.stored && &m.name == p).and_then(|m| m.data(sarc)) {
            Some(d) => { out.insert(p.clone(), d.to_vec()); }
            None => { external.insert(hashlittle(p.as_bytes(), 0), p.clone()); }
        }
    }
    if let Some(gd) = game_dir {
        for tab in discover_tabs(gd) {
            if external.is_empty() { break; }
            let tb = match std::fs::read(&tab) { Ok(b) => b, Err(_) => continue };
            let t = match parse_tab(&tb) { Ok(t) => t, Err(_) => continue };
            if !t.entries.iter().any(|e| external.contains_key(&e.name_hash)) { continue; }
            let mut f = match File::open(tab.with_extension("arc")) { Ok(f) => f, Err(_) => continue };
            let mut oodle = None;
            for e in &t.entries {
                if let Some(path) = external.get(&e.name_hash).cloned() {
                    if let Ok(d) = decode_entry(&mut f, &t, e, dll, &mut oodle) { out.insert(path.clone(), d); external.remove(&e.name_hash); }
                }
            }
        }
    }
    out
}

/// Decode a resource path (with its `.hmddsc` sibling) into the pool, deduped; return the pool index.
fn resolve_one(pool: &mut Vec<jc4_formats::amf::Texture>, idx: &mut HashMap<String, Option<usize>>,
               bytes: &HashMap<String, Vec<u8>>, path: &Option<String>) -> Option<usize> {
    let p = path.as_ref()?;
    if let Some(&i) = idx.get(p) { return i; }
    let tex = bytes.get(p).and_then(|dd| jc4_formats::avtx::decode_rgba_from(dd, bytes.get(&hmddsc_of(p)).map(|v| v.as_slice())).ok())
        .map(|(w, h, rgba)| jc4_formats::amf::Texture { w: w as usize, h: h as usize, rgba });
    if tex.is_none() { eprintln!("  tex MISSING {p}"); }
    let slot = tex.map(|t| { pool.push(t); pool.len() - 1 });
    idx.insert(p.clone(), slot);
    slot
}

/// Resolve each submesh's CarPaint textures (`_dif`/`_nrm`/`_mpm`, each with `.hmddsc` hi-res) into a pool.
fn resolve_model_textures(sarc: &[u8], mesh: &jc4_formats::amf::Mesh, game_dir: Option<&str>, dll: &str)
    -> (Vec<jc4_formats::amf::Texture>, Vec<jc4_formats::amf::SubTex>) {
    let mut needed = std::collections::BTreeSet::new();
    for sm in &mesh.submeshes {
        for p in [&sm.diffuse, &sm.normal, &sm.mpm].into_iter().flatten() {
            needed.insert(p.clone());
            needed.insert(hmddsc_of(p));
        }
    }
    let bytes = fetch_paths(&needed, sarc, game_dir, dll);
    let mut pool = Vec::new();
    let mut idx: HashMap<String, Option<usize>> = HashMap::new();
    let sub = mesh.submeshes.iter().map(|sm| jc4_formats::amf::SubTex {
        dif: resolve_one(&mut pool, &mut idx, &bytes, &sm.diffuse),
        nrm: resolve_one(&mut pool, &mut idx, &bytes, &sm.normal),
        mpm: resolve_one(&mut pool, &mut idx, &bytes, &sm.mpm),
    }).collect();
    (pool, sub)
}

// Auto-discover renderable model units: for every entry that resolves to a `.ee` entity, decode it and
// check its SARC for `.modelc` parts. Emits highlights-format lines (`MODEL | label | archive | path`)
// so the workshop's HIGHLIGHTS panel can populate itself — no hand-curation.
fn cmd_discover(game_dir: &str, out_path: &str, filelist: &str, only: Option<&str>) {
    let names = load_filelist_map(filelist);
    let dll = oodle_dll_from_env();
    let root = std::path::Path::new(game_dir).join("archives_win64");
    let mut out: Vec<(String, String, String, usize)> = Vec::new(); // (arc_label, label, path, parts)
    let mut scanned = 0u32;
    for tab_path in discover_tabs(game_dir) {
        let arc_label = tab_path.strip_prefix(&root).ok()
            .and_then(|r| r.with_extension("").to_str().map(|s| s.replace('\\', "/")))
            .unwrap_or_default();
        let b = match std::fs::read(&tab_path) { Ok(b) => b, Err(_) => continue };
        let t = match parse_tab(&b) { Ok(t) => t, Err(_) => continue };
        let arc_path = tab_path.with_extension("arc");
        let mut arc = match File::open(&arc_path) { Ok(f) => f, Err(_) => continue };
        let mut oodle: Option<Oodle> = None;
        for e in &t.entries {
            let path = match names.get(&e.name_hash) { Some(p) if p.ends_with(".ee") => p, _ => continue };
            if let Some(s) = only { if !path.contains(s) { continue; } }
            let data = match decode_entry(&mut arc, &t, e, &dll, &mut oodle) { Ok(d) => d, Err(_) => continue };
            if magic_ext(&data) != "sarc" { continue; }
            let members = match jc4_formats::sarc::parse(&data) { Ok(m) => m, Err(_) => continue };
            let parts = members.iter().filter(|m| m.stored && m.name.ends_with(".modelc")).count();
            if parts == 0 { continue; }
            let label = basename(path).trim_end_matches(".ee").to_string();
            out.push((arc_label.clone(), label, path.clone(), parts));
        }
        scanned += 1;
        eprintln!("[{scanned}] {arc_label}  ({} models so far)", out.len());
    }
    out.sort();
    let mut s = String::from("# auto-discovered model units (jc4_arc discover). regenerate anytime.\n");
    for (arc, label, path, parts) in &out {
        s.push_str(&format!("MODEL | {label} ({parts}p) | {arc} | {path}\n"));
    }
    match std::fs::write(out_path, &s) {
        Ok(_) => println!("discovered {} model units -> {out_path}", out.len()),
        Err(x) => eprintln!("write {out_path}: {x}"),
    }
}

fn discover_tabs(game_dir: &str) -> Vec<std::path::PathBuf> {
    let root = std::path::Path::new(game_dir).join("archives_win64");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { stack.push(p); }
                else if p.extension().map_or(false, |x| x == "tab") { out.push(p); }
            }
        }
    }
    out.sort();
    out
}

// Build a name dictionary by (a) merging any gibbed filelists under `--gibbed DIR`, then (b) harvesting
// path strings embedded in every decoded entry across the game's archives. Writes unique paths (one per
// line) to `out` — a filelist the workshop loads directly — and reports how many real entry hashes the
// merged dictionary now resolves. `--only SUB` limits to archives whose path contains SUB (fast scoping).
fn cmd_dict(game_dir: &str, out_path: &str, gibbed: Option<&str>, only: Option<&str>) {
    use std::collections::BTreeSet;
    let dll = oodle_dll_from_env();
    let mut paths: BTreeSet<String> = BTreeSet::new();
    // (a) merge gibbed filelists
    if let Some(g) = gibbed {
        let mut stack = vec![std::path::PathBuf::from(g)];
        let (mut files, mut merged) = (0u32, 0usize);
        while let Some(p) = stack.pop() {
            if p.is_dir() { if let Ok(rd) = std::fs::read_dir(&p) { for e in rd.flatten() { stack.push(e.path()); } } }
            else if let Ok(txt) = std::fs::read_to_string(&p) {
                files += 1;
                for line in txt.lines() {
                    let l = line.trim();
                    if !l.is_empty() && !l.starts_with(';') { paths.insert(l.to_ascii_lowercase().replace('\\', "/")); merged += 1; }
                }
            }
        }
        eprintln!("gibbed: merged {merged} lines from {files} filelist(s) -> {} unique", paths.len());
    }
    // (b) self-harvest across archives
    let tabs = discover_tabs(game_dir);
    let mut entry_hashes: HashSet<u32> = HashSet::new();
    let mut oodle: Option<Oodle> = None;
    let mut scanned = 0u32;
    for tab_path in &tabs {
        let label = tab_path.to_string_lossy().replace('\\', "/");
        if let Some(sub) = only { if !label.to_lowercase().contains(&sub.to_lowercase()) { continue; } }
        let arc_path = tab_path.with_extension("arc");
        let b = match std::fs::read(tab_path) { Ok(b) => b, Err(_) => continue };
        let t = match parse_tab(&b) { Ok(t) => t, Err(_) => continue };
        let mut arc = match File::open(&arc_path) { Ok(f) => f, Err(_) => continue };
        let before = paths.len();
        for e in &t.entries {
            entry_hashes.insert(e.name_hash);
            if let Ok(data) = decode_entry(&mut arc, &t, e, &dll, &mut oodle) { harvest_paths(&data, &mut paths); }
        }
        scanned += 1;
        eprintln!("[{scanned}] {}  (+{} paths, {} total)",
            tab_path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            paths.len() - before, paths.len());
    }
    // coverage: how many real entry hashes does the merged dictionary resolve?
    let dict_hashes: HashSet<u32> = paths.iter().map(|p| hashlittle(p.as_bytes(), 0)).collect();
    let covered = entry_hashes.iter().filter(|h| dict_hashes.contains(h)).count();
    let total = entry_hashes.len().max(1);
    // write
    let mut s = String::with_capacity(paths.len() * 48);
    for p in &paths { s.push_str(p); s.push('\n'); }
    match std::fs::write(out_path, &s) {
        Ok(_) => println!("wrote {} unique paths -> {out_path}", paths.len()),
        Err(x) => { eprintln!("write {out_path}: {x}"); return; }
    }
    println!("coverage: {covered}/{total} entry hashes resolved ({:.1}%) across {scanned} archive(s)",
        100.0 * covered as f64 / total as f64);
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
        eprintln!("  jc4_arc bundle <resourcebundle>                 list members + summarize a structure");
        eprintln!("  jc4_arc sarc   <sarc/.ee/model>                 list a SARC's grouped member files");
        eprintln!("  jc4_arc model  <sarc>                           model assembly: LODs/materials/textures/mesh");
        eprintln!("  jc4_arc extract <tab> <arc> <outdir> [limit]    decode payloads (raw/zlib/Oodle)");
        eprintln!("  jc4_arc dict   <game_dir> <out.filelist> [--gibbed DIR] [--only SUB]");
        eprintln!("                                                 build a name dictionary (harvest embedded paths + merge gibbed)");
        return;
    }
    match a[1].as_str() {
        "header" => cmd_header(&a[2]),
        "verify" => cmd_verify(&a[2], a.get(3).map(|s| s.as_str())),
        "list" => cmd_list(&a[2], a.get(3).and_then(|s| s.parse().ok()).unwrap_or(32)),
        "hex" => cmd_hex(&a[2], a.get(3).and_then(|s| s.parse().ok()).unwrap_or(256)),
        "hash" => println!("{:08x}  {:?}", hashlittle(a[2].as_bytes(), 0), a[2]),
        "bundle" => cmd_bundle(&a[2]),
        "sarc" => cmd_sarc(&a[2], a.get(3).map(|s| s.as_str())),
        "model" => {
            if let Some(png) = a.iter().position(|s| s == "--png").and_then(|i| a.get(i + 1)) {
                let b = std::fs::read(&a[2]).unwrap();
                let epe = a.iter().position(|s| s == "--epe").and_then(|i| a.get(i + 1)).map(|p| std::fs::read(p).unwrap());
                match jc4_formats::amf::decode_model_asm(&b, epe.as_deref(), jc4_formats::amf::PartSel::default()) {
                    Ok(m) => {
                        let yaw = a.iter().position(|s| s == "--yaw").and_then(|i| a.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(0.7);
                        let pitch = a.iter().position(|s| s == "--pitch").and_then(|i| a.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(0.3);
                        let game = a.iter().position(|s| s == "--game").and_then(|i| a.get(i + 1)).map(|s| s.as_str());
                        let (pool, sub) = if a.iter().any(|s| s == "--uvcheck") {
                            // synthetic checkerboard on every submesh — clean squares ⇒ UVs are fine
                            let (tw, th) = (256usize, 256usize);
                            let mut rgba = vec![0u8; tw * th * 4];
                            for y in 0..th { for x in 0..tw {
                                let c = if ((x / 16) + (y / 16)) % 2 == 0 { [230, 60, 60, 255] } else { [240, 240, 240, 255] };
                                rgba[(y * tw + x) * 4..(y * tw + x) * 4 + 4].copy_from_slice(&c);
                            }}
                            (vec![jc4_formats::amf::Texture { w: tw, h: th, rgba }],
                             m.submeshes.iter().map(|_| jc4_formats::amf::SubTex { dif: Some(0), nrm: None, mpm: None }).collect::<Vec<_>>())
                        } else {
                            resolve_model_textures(&b, &m, game, &oodle_dll_from_env())
                        };
                        let ntex = pool.len();
                        let size = a.iter().position(|s| s == "--size").and_then(|i| a.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(512);
                        std::fs::write(png, jc4_formats::amf::render_png_tex(&m, size, yaw, pitch, &pool, &sub)).unwrap();
                        println!("merged model: {} verts, {} tris, {ntex} textures -> {png}", m.positions.len(), m.indices.len() / 3);
                    }
                    Err(e) => eprintln!("decode_model: {e}"),
                }
            } else { cmd_model(&a[2]); }
        }
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
            cmd_extract(&a[2], &a[3], &a[4], limit, &oodle_dll_from_env(), fl.as_ref(), msub.as_deref());
        }
        "mesh" => {
            if a.len() < 3 { eprintln!("usage: jc4_arc mesh <meshc> [out.obj] [--hr <hrmeshc>]"); return; }
            let b = std::fs::read(&a[2]).unwrap();
            let hr = a.iter().position(|s| s == "--hr").and_then(|i| a.get(i + 1)).map(|p| std::fs::read(p).unwrap());
            let decoded = match &hr {
                Some(h) => jc4_formats::amf::decode_mesh_hr(&b, h),
                None => jc4_formats::amf::decode_mesh(&b),
            };
            match decoded {
                Ok(m) => {
                    let (mut umin, mut umax, mut vmin, mut vmax) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
                    for uv in &m.uvs { umin = umin.min(uv[0]); umax = umax.max(uv[0]); vmin = vmin.min(uv[1]); vmax = vmax.max(uv[1]); }
                    println!("mesh: {} verts, {} tris  bbox min {:?} max {:?}  UV u[{:.2},{:.2}] v[{:.2},{:.2}]",
                        m.positions.len(), m.indices.len() / 3, m.bbox_min, m.bbox_max, umin, umax, vmin, vmax);
                    if let Some(out) = a.get(3).filter(|s| !s.starts_with("--")) {
                        std::fs::write(out, jc4_formats::amf::to_obj(&m)).unwrap();
                        println!("wrote {out}");
                    }
                    if let Some(png) = a.iter().position(|s| s == "--png").and_then(|i| a.get(i + 1)) {
                        let yaw = a.iter().position(|s| s == "--yaw").and_then(|i| a.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(0.6);
                        let pitch = a.iter().position(|s| s == "--pitch").and_then(|i| a.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(0.35);
                        std::fs::write(png, jc4_formats::amf::render_png(&m, 512, yaw, pitch)).unwrap();
                        println!("rendered {png}");
                    }
                }
                Err(e) => eprintln!("decode failed: {e}"),
            }
        }
        "dict" => {
            if a.len() < 4 { eprintln!("usage: jc4_arc dict <game_dir> <out.filelist> [--gibbed DIR] [--only SUB]"); return; }
            let gibbed = a.iter().position(|s| s == "--gibbed").and_then(|i| a.get(i + 1)).map(|s| s.as_str());
            let only = a.iter().position(|s| s == "--only").and_then(|i| a.get(i + 1)).map(|s| s.as_str());
            cmd_dict(&a[2], &a[3], gibbed, only);
        }
        "discover" => {
            if a.len() < 5 { eprintln!("usage: jc4_arc discover <game_dir> <out.txt> --filelist F [--only SUB]"); return; }
            let fl = a.iter().position(|s| s == "--filelist").and_then(|i| a.get(i + 1));
            let only = a.iter().position(|s| s == "--only").and_then(|i| a.get(i + 1)).map(|s| s.as_str());
            match fl { Some(f) => cmd_discover(&a[2], &a[3], f, only), None => eprintln!("--filelist required") }
        }
        other => eprintln!("unknown command {other}"),
    }
}
