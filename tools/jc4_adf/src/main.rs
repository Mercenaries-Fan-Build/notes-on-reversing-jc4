// jc4_adf — Just Cause 4 ADF (Avalanche Data Format) typed decoder.
//
// ADF is Avalanche's reflection-based typed binary container (most JC4 assets). Format cracked
// double-blind (2026-08-15, two investigators converged; 90/90 corpus files decode). Decomp oracle:
// header FUN_14aafceb0, instance table FUN_14aaf6c70, decoder FUN_140f2e0d0, metatype dispatch
// FUN_140f2c440. Full spec: ../../docs/formats/adf.md.
//
// Layout: 64-byte header (magic " FDA"=0x41444620, version 4, {instance,typedef,stringhash,name}
// count+offset pairs, total_size==filelen), comment; then (in any order, offsets are authoritative)
// the type-def table, instance table, string-hash table, name table. Types & member names by hash.
// All little-endian.

use std::collections::HashMap;
use serde_json::{json, Value};

fn u16le(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o+1], b[o+2], b[o+3]]) }
fn u64le(b: &[u8], o: usize) -> u64 { u64::from_le_bytes(b[o..o+8].try_into().unwrap()) }
fn i32le(b: &[u8], o: usize) -> i32 { i32::from_le_bytes([b[o], b[o+1], b[o+2], b[o+3]]) }

const MAGIC: u32 = 0x41444620; // " FDA"

#[derive(Clone)]
struct Member { name_index: u64, type_hash: u32, offset_packed: u32 }
#[derive(Clone)]
struct TypeDef {
    metatype: u32, size: u32, type_hash: u32, name_index: u64,
    scalar_type: u16, sub_type_hash: u32, elem_len: u32,
    members: Vec<Member>, enum_entries: Vec<(u64, i32)>,
}

struct Adf {
    data: Vec<u8>,
    names: Vec<String>,
    types: HashMap<u32, TypeDef>,
    strhash: HashMap<u32, String>,
    instances: Vec<(u32, u32, u32, u32, u64)>, // name_hash, type_hash, off, size, name_index
    total_size: u32,
}

// (width, kind) — kind: 0=signed,1=unsigned,2=float ; String handled specially.
fn builtin(hash: u32) -> Option<(usize, u8)> {
    Some(match hash {
        0x580D0A62 => (1, 0), 0x0CA2821D => (1, 1),   // int8, uint8
        0xD13FCF93 => (2, 0), 0x86D152BD => (2, 1),   // int16, uint16
        0x192FE633 => (4, 0), 0x075E4E4F => (4, 1),   // int32, uint32
        0xAF41354F => (8, 0), 0xA139E01F => (8, 1),   // int64, uint64
        0x7515A207 => (4, 2), 0xC609F663 => (8, 2),   // float, double
        _ => return None,
    })
}
const STRING_HASH: u32 = 0x8955583E; // deferred string ref (8-byte inline offset)
const DEFERRED_HASH: u32 = 0xDEFE88ED; // deferred/polymorphic ref: {offset, flags, type_hash, _}

fn cstr(b: &[u8], o: usize) -> String {
    let end = b[o..].iter().position(|&c| c == 0).map(|p| o + p).unwrap_or(b.len());
    String::from_utf8_lossy(&b[o..end]).into_owned()
}

fn parse(data: Vec<u8>) -> Result<Adf, String> {
    if data.len() < 0x40 || u32le(&data, 0) != MAGIC {
        return Err("not an ADF (bad magic)".into());
    }
    let instance_count = u32le(&data, 0x08); let instance_off = u32le(&data, 0x0C) as usize;
    let typedef_count = u32le(&data, 0x10); let typedef_off = u32le(&data, 0x14) as usize;
    let strhash_count = u32le(&data, 0x18); let strhash_off = u32le(&data, 0x1C) as usize;
    let name_count = u32le(&data, 0x20); let name_off = u32le(&data, 0x24) as usize;
    let total_size = u32le(&data, 0x28);

    // name table: name_count length bytes, then that many NUL-terminated strings
    let mut names = Vec::with_capacity(name_count as usize);
    let mut p = name_off + name_count as usize;
    for _ in 0..name_count {
        let s = cstr(&data, p);
        p += s.len() + 1;
        names.push(s);
    }
    // string-hash table: {cstring, u32 hash, u32 pad}
    let mut strhash = HashMap::new();
    let mut p = strhash_off;
    for _ in 0..strhash_count {
        let s = cstr(&data, p); p += s.len() + 1;
        let h = u32le(&data, p); p += 8; // u32 hash + u32(=0)
        strhash.insert(h, s);
    }
    // type-def table
    let mut types = HashMap::new();
    let mut p = typedef_off;
    for _ in 0..typedef_count {
        let metatype = u32le(&data, p);
        let td = TypeDef {
            metatype,
            size: u32le(&data, p + 0x04),
            type_hash: u32le(&data, p + 0x0C),
            name_index: u64le(&data, p + 0x10),
            scalar_type: u16le(&data, p + 0x1A),
            sub_type_hash: u32le(&data, p + 0x1C),
            elem_len: u32le(&data, p + 0x20),
            members: Vec::new(),
            enum_entries: Vec::new(),
        };
        let member_count = u32le(&data, p + 0x24) as usize;
        p += 0x28;
        let mut td = td;
        if metatype == 1 {
            for _ in 0..member_count {
                td.members.push(Member {
                    name_index: u64le(&data, p),
                    type_hash: u32le(&data, p + 0x08),
                    offset_packed: u32le(&data, p + 0x10),
                });
                p += 0x20;
            }
        } else if metatype == 8 {
            for _ in 0..member_count {
                td.enum_entries.push((u64le(&data, p), i32le(&data, p + 0x08)));
                p += 0x0C;
            }
        }
        types.insert(td.type_hash, td);
    }
    // instance table (v4 stride 0x18)
    let mut instances = Vec::with_capacity(instance_count as usize);
    for i in 0..instance_count as usize {
        let o = instance_off + i * 0x18;
        instances.push((u32le(&data, o), u32le(&data, o+4), u32le(&data, o+8),
                        u32le(&data, o+12), u64le(&data, o+16)));
    }
    Ok(Adf { data, names, types, strhash, instances, total_size })
}

impl Adf {
    fn name(&self, idx: u64) -> &str { self.names.get(idx as usize).map(|s| s.as_str()).unwrap_or("?") }

    fn type_size(&self, hash: u32) -> usize {
        if let Some((w, _)) = builtin(hash) { return w; }
        if hash == STRING_HASH { return 8; }
        self.types.get(&hash).map(|t| t.size as usize).unwrap_or(0)
    }

    // Decode value of `hash` at absolute file offset `off`; `base` = root instance payload offset.
    fn decode(&self, hash: u32, off: usize, base: usize, bit: u32, depth: u32) -> Value {
        if depth > 64 { return json!("<max-depth>"); }
        let d = &self.data;
        if let Some((w, kind)) = builtin(hash) {
            return match (w, kind) {
                (_, 2) if w == 4 => json!(f32::from_le_bytes(d[off..off+4].try_into().unwrap())),
                (_, 2) => json!(f64::from_le_bytes(d[off..off+8].try_into().unwrap())),
                (1, 0) => json!(d[off] as i8), (1, 1) => json!(d[off]),
                (2, 0) => json!(u16le(d, off) as i16), (2, 1) => json!(u16le(d, off)),
                (4, 0) => json!(i32le(d, off)), (4, 1) => json!(u32le(d, off)),
                (8, 0) => json!(u64le(d, off) as i64), (8, 1) => json!(u64le(d, off)),
                _ => json!(null),
            };
        }
        if hash == STRING_HASH {
            let so = u32le(d, off) as usize;
            return if so == 0 { json!("") } else { json!(cstr(d, base + so)) };
        }
        if hash == DEFERRED_HASH {
            // {offset u32, flags u32, type_hash u32, _} → recurse at base+offset with the stored type
            let (so, th) = (u32le(d, off) as usize, u32le(d, off + 8));
            return if so == 0 || th == 0 { json!(null) } else { self.decode(th, base + so, base, 0, depth + 1) };
        }
        let t = match self.types.get(&hash) { Some(t) => t, None => return json!(format!("<type 0x{hash:08x}>")) };
        match t.metatype {
            0 => { // primitive typedef
                match (t.size as usize, t.scalar_type) {
                    (4, 2) => json!(f32::from_le_bytes(d[off..off+4].try_into().unwrap())),
                    (8, 2) => json!(f64::from_le_bytes(d[off..off+8].try_into().unwrap())),
                    (4, 0) => json!(i32le(d, off)), (4, 1) => json!(u32le(d, off)),
                    _ => json!(null),
                }
            }
            1 => { // struct
                let mut m = serde_json::Map::new();
                for mem in &t.members {
                    let real = off + (mem.offset_packed & 0xFFFFFF) as usize;
                    let mbit = mem.offset_packed >> 24;
                    m.insert(self.name(mem.name_index).to_string(),
                             self.decode(mem.type_hash, real, base, mbit, depth + 1));
                }
                Value::Object(m)
            }
            2 => { // pointer
                let p = u32le(d, off) as usize;
                if p == 0 { json!(null) } else { self.decode(t.sub_type_hash, base + p, base, 0, depth + 1) }
            }
            3 => { // array (dynamic): {offset u32, reloc u32, count u32, _}
                let ao = u32le(d, off) as usize;
                let count = u32le(d, off + 8) as usize;
                let stride = self.type_size(t.sub_type_hash).max(1);
                let mut arr = Vec::with_capacity(count);
                for i in 0..count {
                    arr.push(self.decode(t.sub_type_hash, base + ao + i * stride, base, 0, depth + 1));
                }
                Value::Array(arr)
            }
            4 => { // inline array
                let stride = self.type_size(t.sub_type_hash).max(1);
                let mut arr = Vec::with_capacity(t.elem_len as usize);
                for i in 0..t.elem_len as usize {
                    arr.push(self.decode(t.sub_type_hash, off + i * stride, base, 0, depth + 1));
                }
                Value::Array(arr)
            }
            5 => { // string (inline offset)
                let so = u32le(d, off) as usize;
                if so == 0 { json!("") } else { json!(cstr(d, base + so)) }
            }
            7 => { // bitfield: extract elem_len bits at `bit` from a size-byte cell
                let cell = match t.size { 1 => d[off] as u64, 2 => u16le(d, off) as u64,
                    4 => u32le(d, off) as u64, _ => u64le(d, off) };
                let mask = if t.elem_len >= 64 { u64::MAX } else { (1u64 << t.elem_len) - 1 };
                json!((cell >> bit) & mask)
            }
            8 => { // enum
                let v = i32le(d, off);
                let name = t.enum_entries.iter().find(|(_, ev)| *ev == v).map(|(n, _)| self.name(*n).to_string());
                match name { Some(n) => json!(n), None => json!(v) }
            }
            9 => { // stringhash
                let h = u32le(d, off);
                match self.strhash.get(&h) { Some(s) => json!(s), None => json!(format!("#{h:08x}")) }
            }
            10 => { // deferred/polymorphic: {offset, flags, type_hash, _}
                let (so, th) = (u32le(d, off) as usize, u32le(d, off + 8));
                if so == 0 || th == 0 { json!(null) } else { self.decode(th, base + so, base, 0, depth + 1) }
            }
            _ => json!(format!("<meta {}>", t.metatype)),
        }
    }

    fn decode_instances(&self) -> Value {
        let mut out = serde_json::Map::new();
        for &(name_hash, type_hash, off, _size, name_index) in &self.instances {
            let key = if (name_index as usize) < self.names.len() {
                self.name(name_index).to_string()
            } else { format!("{name_hash:08x}") };
            out.insert(key, self.decode(type_hash, off as usize, off as usize, 0, 0));
        }
        Value::Object(out)
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("jc4_adf — ADF typed decoder");
        eprintln!("  jc4_adf info <adf>          header + type/instance summary");
        eprintln!("  jc4_adf dump <adf>          decode instances -> JSON (stdout)");
        eprintln!("  jc4_adf verify <dir>        parse+decode every *.adf under <dir>, report pass/fail");
        return;
    }
    match a[1].as_str() {
        "info" => {
            let adf = parse(std::fs::read(&a[2]).unwrap()).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1) });
            println!("total_size={} (file={})  types={} instances={} names={} strhash={}",
                adf.total_size, adf.data.len(), adf.types.len(), adf.instances.len(), adf.names.len(), adf.strhash.len());
            for &(nh, th, off, size, ni) in &adf.instances {
                println!("  instance {} : type 0x{th:08x} @0x{off:x} size {size} (name_hash 0x{nh:08x}, name_idx {ni})",
                    adf.name(ni));
            }
        }
        "dump" => {
            let adf = parse(std::fs::read(&a[2]).unwrap()).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1) });
            println!("{}", serde_json::to_string_pretty(&adf.decode_instances()).unwrap());
        }
        "verify" => {
            let dir = &a[2];
            let (mut ok, mut bad) = (0u32, 0u32);
            let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map_or(false, |x| x == "adf")).collect();
            files.sort();
            for f in &files {
                match std::panic::catch_unwind(|| {
                    let adf = parse(std::fs::read(f).unwrap())?;
                    if adf.total_size as usize != adf.data.len() { return Err("total_size!=len".to_string()); }
                    let _ = adf.decode_instances(); // must not panic / OOB
                    Ok::<(), String>(())
                }) {
                    Ok(Ok(())) => ok += 1,
                    Ok(Err(e)) => { bad += 1; eprintln!("FAIL {}: {e}", f.display()); }
                    Err(_) => { bad += 1; eprintln!("PANIC {}", f.display()); }
                }
            }
            println!("ADF verify: PASS={ok} FAIL={bad} / {} files", files.len());
        }
        other => eprintln!("unknown command {other}"),
    }
}
