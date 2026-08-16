//! GPU combinator: generate candidate paths ON the device (mixed-radix over vocabulary slots), hash each
//! with a WGSL port of lookup3 (byte-for-byte the Rust `hashlittle`), and binary-search the sorted target
//! hashes — so we never stream billions of strings over PCIe, only the vocabulary + a template. This is
//! the scale lever the CPU crack caps out of (VOCAB_CAP): here every token in a context is fair game.
//! Hits (winning combination indices) come back tiny; the CPU reconstructs + re-verifies each string.
use std::collections::BTreeSet;

use jc4_formats::hash::hashlittle;
use jc4_formats::tab::parse_tab;
use wgpu::util::DeviceExt;

const WGSL: &str = r#"
struct Meta { num_parts:u32, num_slots:u32, base:u32, total:u32, out_cap:u32, ntarget:u32, _p0:u32, _p1:u32 };
@group(0) @binding(0) var<uniform> gm: Meta;
@group(0) @binding(1) var<storage,read> parts: array<vec4<u32>>;   // (kind,a,b,_) kind0=literal a=off b=len ; kind1=slot a=slotIndex
@group(0) @binding(2) var<storage,read> slots: array<vec2<u32>>;   // (first_token, count)
@group(0) @binding(3) var<storage,read> tok_off: array<vec2<u32>>; // (byte_start, len) per token
@group(0) @binding(4) var<storage,read> vbytes: array<u32>;        // vocab bytes (one byte per u32)
@group(0) @binding(5) var<storage,read> lit: array<u32>;           // literal bytes (one byte per u32)
@group(0) @binding(6) var<storage,read> targets: array<u32>;       // sorted ascending
@group(0) @binding(7) var<storage,read_write> out_count: atomic<u32>;
@group(0) @binding(8) var<storage,read_write> out_ids: array<u32>;

var<private> b8: array<u32, 224>;

fn rot(x:u32,k:u32)->u32 { return (x << k) | (x >> (32u - k)); }
fn rd(o:u32)->u32 { return b8[o] | (b8[o+1u]<<8u) | (b8[o+2u]<<16u) | (b8[o+3u]<<24u); }

fn hashlittle(len0:u32)->u32 {
  var len = len0;
  let init = 0xdeadbeefu + len0;           // + initval(0)
  var a=init; var b=init; var c=init;
  var off=0u;
  loop {
    if (len <= 12u) { break; }
    a = a + rd(off); b = b + rd(off+4u); c = c + rd(off+8u);
    a=a-c; a=a^rot(c,4u);  c=c+b;
    b=b-a; b=b^rot(a,6u);  a=a+c;
    c=c-b; c=c^rot(b,8u);  b=b+a;
    a=a-c; a=a^rot(c,16u); c=c+b;
    b=b-a; b=b^rot(a,19u); a=a+c;
    c=c-b; c=c^rot(b,4u);  b=b+a;
    off=off+12u; len=len-12u;
  }
  var t: array<u32,12>;
  for (var i=0u;i<12u;i=i+1u){ t[i]=0u; }
  for (var i=0u;i<len;i=i+1u){ t[i]=b8[off+i]; }
  a = a + (t[0]|(t[1]<<8u)|(t[2]<<16u)|(t[3]<<24u));
  b = b + (t[4]|(t[5]<<8u)|(t[6]<<16u)|(t[7]<<24u));
  c = c + (t[8]|(t[9]<<8u)|(t[10]<<16u)|(t[11]<<24u));
  if (len0 == 0u) { return c; }
  c=c^b; c=c-rot(b,14u);
  a=a^c; a=a-rot(c,11u);
  b=b^a; b=b-rot(a,25u);
  c=c^b; c=c-rot(b,16u);
  a=a^c; a=a-rot(c,4u);
  b=b^a; b=b-rot(a,14u);
  c=c^b; c=c-rot(b,24u);
  return c;
}

fn is_target(h:u32)->bool {
  var lo=0u; var hi=gm.ntarget;
  loop {
    if (lo>=hi) { break; }
    let mid=(lo+hi)/2u;
    let v=targets[mid];
    if (v==h) { return true; }
    if (v<h) { lo=mid+1u; } else { hi=mid; }
  }
  return false;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let idx = gm.base + gid.x;
  if (idx >= gm.total) { return; }
  // mixed-radix decode of idx -> chosen token per slot
  var choice: array<u32,8>;
  var rem = idx;
  var s = gm.num_slots;
  loop {
    if (s == 0u) { break; }
    s = s - 1u;
    let cnt = slots[s].y;
    choice[s] = rem % cnt;
    rem = rem / cnt;
  }
  // assemble bytes per parts
  var pos = 0u;
  for (var p=0u; p<gm.num_parts; p=p+1u) {
    let part = parts[p];
    if (part.x == 0u) {
      for (var i=0u; i<part.z; i=i+1u) { b8[pos] = lit[part.y+i]; pos=pos+1u; }
    } else {
      let sl = slots[part.y];
      let to = tok_off[sl.x + choice[part.y]];
      for (var i=0u; i<to.y; i=i+1u) { b8[pos] = vbytes[to.x+i]; pos=pos+1u; }
    }
  }
  let h = hashlittle(pos);
  if (is_target(h)) {
    let o = atomicAdd(&out_count, 1u);
    if (o < gm.out_cap) { out_ids[o] = idx; }
  }
}
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Meta { num_parts: u32, num_slots: u32, base: u32, total: u32, out_cap: u32, ntarget: u32, _p0: u32, _p1: u32 }

enum Part { Lit(String), Slot(usize) }

/// A generation template: ordered literal/slot parts, and each slot's token list.
struct Template { parts: Vec<Part>, slots: Vec<Vec<String>> }

impl Template {
    fn total(&self) -> u64 { self.slots.iter().map(|s| s.len() as u64).product() }
    /// CPU mirror of the WGSL assembly — reconstruct the string for a combination index.
    fn build(&self, mut idx: u64) -> String {
        let mut choice = vec![0usize; self.slots.len()];
        for s in (0..self.slots.len()).rev() {
            let cnt = self.slots[s].len() as u64;
            choice[s] = (idx % cnt) as usize;
            idx /= cnt;
        }
        let mut out = String::new();
        for p in &self.parts {
            match p { Part::Lit(s) => out.push_str(s), Part::Slot(i) => out.push_str(&self.slots[*i][choice[*i]]) }
        }
        out
    }
}

fn discover_tabs(game_dir: &str) -> Vec<std::path::PathBuf> {
    let root = std::path::Path::new(game_dir).join("archives_win64");
    let (mut out, mut stack) = (Vec::new(), vec![root]);
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { stack.push(p); }
                else if p.extension().map_or(false, |x| x == "tab") { out.push(p); }
            }
        }
    }
    out
}

pub fn run(game_dir: &str, dict_path: &str, out_path: &str, ctx: &str) {
    // target hashes, sorted
    let mut tset: BTreeSet<u32> = BTreeSet::new();
    for tab in discover_tabs(game_dir) {
        if let Ok(b) = std::fs::read(&tab) { if let Ok(t) = parse_tab(&b) { for e in &t.entries { tset.insert(e.name_hash); } } }
    }
    let targets: Vec<u32> = tset.iter().copied().collect();
    eprintln!("targets: {}", targets.len());

    // build the context vocabulary (ALL tokens — no cap) + extensions, from the dict
    let known = crate::read_lines(dict_path);
    let mut toks: BTreeSet<String> = BTreeSet::new();
    let mut exts: BTreeSet<String> = BTreeSet::new();
    for p in &known {
        if !p.starts_with(&format!("{ctx}/")) { continue; }
        let file = p.rsplit('/').next().unwrap_or(p);
        let base = file.split('.').next().unwrap_or(file);
        for t in base.split(['_', '-']) { if t.len() >= 3 && !t.bytes().all(|b| b.is_ascii_digit()) { toks.insert(t.to_string()); } }
        if let Some((_, e)) = file.split_once('.') { exts.insert(e.to_string()); }
    }
    let tok: Vec<String> = toks.into_iter().collect();
    let ext: Vec<String> = exts.into_iter().collect();
    if tok.is_empty() || ext.is_empty() { eprintln!("empty vocabulary for context '{ctx}'"); return; }

    // template: CTX/ <tok> _ <tok> . <ext>   (two-token basenames — the pattern the CPU capped)
    let tmpl = Template {
        parts: vec![Part::Lit(format!("{ctx}/")), Part::Slot(0), Part::Lit("_".into()), Part::Slot(1), Part::Lit(".".into()), Part::Slot(2)],
        slots: vec![tok.clone(), tok.clone(), ext.clone()],
    };
    let total = tmpl.total();
    eprintln!("context '{ctx}': {} tokens, {} exts -> {} combinations", tok.len(), ext.len(), total);
    if total >= u32::MAX as u64 { eprintln!("total {total} exceeds u32 range; narrow the context (v1 limit)"); return; }

    let hits = dispatch_gpu(&tmpl, &targets, total as u32);
    // reconstruct + re-verify on CPU (guards against any kernel discrepancy)
    let mut cracked: BTreeSet<String> = BTreeSet::new();
    let tset_hash: std::collections::HashSet<u32> = targets.iter().copied().collect();
    let (mut ok, mut bad) = (0u32, 0u32);
    for idx in hits {
        let s = tmpl.build(idx as u64);
        if tset_hash.contains(&hashlittle(s.as_bytes(), 0)) { cracked.insert(s); ok += 1; } else { bad += 1; }
    }
    let mut out = String::new();
    for p in &cracked { out.push_str(p); out.push('\n'); }
    let _ = std::fs::write(out_path, &out);
    println!("GPU hits: {ok} verified, {bad} mismatched-on-recheck; {} unique paths -> {out_path}", cracked.len());
    println!("(feed through `jc4_probe verify` for the content/sibling oracle before trusting)");
}

fn packed_bytes(strs: &[String]) -> (Vec<u32>, Vec<[u32; 2]>) {
    let mut bytes = Vec::new();
    let mut off = Vec::with_capacity(strs.len());
    for s in strs {
        off.push([bytes.len() as u32, s.len() as u32]);
        bytes.extend(s.bytes().map(|b| b as u32));
    }
    (bytes, off)
}

fn dispatch_gpu(tmpl: &Template, targets: &[u32], total: u32) -> Vec<u32> {
    // flatten template for the GPU
    let mut lit: Vec<u32> = Vec::new();
    let mut parts: Vec<[u32; 4]> = Vec::new();
    for p in &tmpl.parts {
        match p {
            Part::Lit(s) => { let o = lit.len() as u32; lit.extend(s.bytes().map(|b| b as u32)); parts.push([0, o, s.len() as u32, 0]); }
            Part::Slot(i) => parts.push([1, *i as u32, 0, 0]),
        }
    }
    // one shared token pool; slots index contiguous ranges
    let mut all_tokens: Vec<String> = Vec::new();
    let mut slots: Vec<[u32; 2]> = Vec::new();
    for s in &tmpl.slots { slots.push([all_tokens.len() as u32, s.len() as u32]); all_tokens.extend_from_slice(s); }
    let (vbytes, tok_off) = packed_bytes(&all_tokens);
    let tok_off: Vec<u32> = tok_off.iter().flat_map(|a| [a[0], a[1]]).collect();
    let slots_f: Vec<u32> = slots.iter().flat_map(|a| [a[0], a[1]]).collect();
    let parts_f: Vec<u32> = parts.iter().flatten().copied().collect();

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default()
    })).expect("no GPU adapter");
    eprintln!("GPU: {}", adapter.get_info().name);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None, required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(), memory_hints: wgpu::MemoryHints::Performance,
    }, None)).expect("device");

    let out_cap: u32 = 1 << 21; // 2M hit slots
    let buf = |data: &[u32], usage| device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None, contents: bytemuck::cast_slice(data), usage,
    });
    use wgpu::BufferUsages as U;
    let b_parts = buf(&parts_f, U::STORAGE);
    let b_slots = buf(&slots_f, U::STORAGE);
    let b_tokoff = buf(&tok_off, U::STORAGE);
    let b_vbytes = buf(&vbytes, U::STORAGE);
    let b_lit = buf(if lit.is_empty() { &[0] } else { &lit }, U::STORAGE);
    let b_targets = buf(targets, U::STORAGE);
    let b_outcount = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: 4, usage: U::STORAGE | U::COPY_SRC | U::COPY_DST, mapped_at_creation: false });
    let b_outids = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (out_cap as u64) * 4, usage: U::STORAGE | U::COPY_SRC, mapped_at_creation: false });
    let b_meta = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: std::mem::size_of::<Meta>() as u64, usage: U::UNIFORM | U::COPY_DST, mapped_at_creation: false });

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(WGSL.into()) });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None, layout: None, module: &module, entry_point: "main",
        compilation_options: Default::default(), cache: None,
    });
    let bgl = pipeline.get_bind_group_layout(0);
    let entries: Vec<wgpu::BindGroupEntry> = [&b_meta, &b_parts, &b_slots, &b_tokoff, &b_vbytes, &b_lit, &b_targets, &b_outcount, &b_outids]
        .iter().enumerate().map(|(i, b)| wgpu::BindGroupEntry { binding: i as u32, resource: b.as_entire_binding() }).collect();
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &bgl, entries: &entries });

    // dispatch in chunks (65535 workgroups * 64 lanes)
    let chunk: u32 = 65535 * 64;
    let mut base = 0u32;
    while base < total {
        let this = (total - base).min(chunk);
        let meta = Meta { num_parts: parts.len() as u32, num_slots: tmpl.slots.len() as u32, base, total, out_cap, ntarget: targets.len() as u32, _p0: 0, _p1: 0 };
        queue.write_buffer(&b_meta, 0, bytemuck::bytes_of(&meta));
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut cp = enc.begin_compute_pass(&Default::default());
            cp.set_pipeline(&pipeline);
            cp.set_bind_group(0, &bind, &[]);
            cp.dispatch_workgroups((this + 63) / 64, 1, 1);
        }
        queue.submit([enc.finish()]);
        device.poll(wgpu::Maintain::Wait);
        base += this;
    }

    // read out_count then out_ids
    let read_u32 = |src: &wgpu::Buffer, n: u64| -> Vec<u32> {
        let staging = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: n * 4, usage: U::COPY_DST | U::MAP_READ, mapped_at_creation: false });
        let mut enc = device.create_command_encoder(&Default::default());
        enc.copy_buffer_to_buffer(src, 0, &staging, 0, n * 4);
        queue.submit([enc.finish()]);
        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);
        let data = slice.get_mapped_range();
        let v: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        drop(data); staging.unmap(); v
    };
    let count = read_u32(&b_outcount, 1)[0];
    let n = count.min(out_cap) as u64;
    eprintln!("GPU raw hits: {count}{}", if count > out_cap { format!(" (capped at {out_cap})") } else { String::new() });
    if n == 0 { return Vec::new(); }
    read_u32(&b_outids, n)
}
