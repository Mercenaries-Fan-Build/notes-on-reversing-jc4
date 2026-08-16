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

const VOCAB_CAP: usize = 1500; // top-N tokens per subtree (per-dir combos = CAP² × |ext|)

/// `--context CTX` (flat: reach assets directly under CTX/) or `--prefix P` (v2: loop every real
/// directory under P, placing generated `<dir>/<tok>_<tok>.<ext>` into the deep tree).
pub fn run(game_dir: &str, dict_path: &str, out_path: &str, selector: &str, prefix_mode: bool) {
    let mut tset: BTreeSet<u32> = BTreeSet::new();
    for tab in discover_tabs(game_dir) {
        if let Ok(b) = std::fs::read(&tab) { if let Ok(t) = parse_tab(&b) { for e in &t.entries { tset.insert(e.name_hash); } } }
    }
    let targets: Vec<u32> = tset.iter().copied().collect();
    eprintln!("targets: {}", targets.len());

    // vocabulary (frequency-ranked tokens), extensions, and the real directories under the subtree
    let known = crate::read_lines(dict_path);
    let under = format!("{}/", selector.trim_end_matches('/'));
    let mut freq: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut exts: BTreeSet<String> = BTreeSet::new();
    let mut dirset: BTreeSet<String> = BTreeSet::new();
    for p in &known {
        if !p.starts_with(&under) { continue; }
        let file = p.rsplit('/').next().unwrap_or(p);
        let base = file.split('.').next().unwrap_or(file);
        for t in base.split(['_', '-']) { if t.len() >= 3 && !t.bytes().all(|b| b.is_ascii_digit()) { *freq.entry(t.to_string()).or_default() += 1; } }
        if let Some((_, e)) = file.split_once('.') { exts.insert(e.to_string()); }
        dirset.insert(p[..p.len() - file.len()].to_string()); // dir incl. trailing '/'
    }
    let mut tv: Vec<(String, u32)> = freq.into_iter().collect();
    tv.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let tok: Vec<String> = tv.into_iter().take(VOCAB_CAP).map(|(k, _)| k).collect();
    let ext: Vec<String> = exts.into_iter().collect();
    if tok.is_empty() || ext.is_empty() { eprintln!("empty vocabulary under '{selector}'"); return; }

    let dirs: Vec<String> = if prefix_mode { dirset.into_iter().collect() } else { vec![under.clone()] };
    let per = tok.len() as u64 * tok.len() as u64 * ext.len() as u64;
    eprintln!("vocab {} tokens, {} exts; {} dir(s); {} combos/dir; {} total",
        tok.len(), ext.len(), dirs.len(), per, per * dirs.len() as u64);
    if per >= u32::MAX as u64 { eprintln!("combos/dir {per} exceeds u32 — lower VOCAB_CAP"); return; }

    let gpu = Gpu::new(&targets, &tok, &ext);
    let tset_hash: std::collections::HashSet<u32> = targets.iter().copied().collect();
    let mut cracked: BTreeSet<String> = BTreeSet::new();
    let (mut done, mut raw) = (0usize, 0u64);
    for d in &dirs {
        let ids = gpu.run_dir(d, per as u32);
        raw += ids.len() as u64;
        let tmpl = Template { parts: dir_parts(d), slots: vec![tok.clone(), tok.clone(), ext.clone()] };
        for idx in ids { let s = tmpl.build(idx as u64); if tset_hash.contains(&hashlittle(s.as_bytes(), 0)) { cracked.insert(s); } }
        done += 1;
        if done % 250 == 0 || done == dirs.len() { eprintln!("  {done}/{} dirs · {} unique cracked", dirs.len(), cracked.len()); }
    }
    let mut out = String::new();
    for p in &cracked { out.push_str(p); out.push('\n'); }
    let _ = std::fs::write(out_path, &out);
    println!("GPU: {raw} raw hits over {} dir(s) -> {} unique verified paths -> {out_path}", dirs.len(), cracked.len());
    println!("(feed through `jc4_probe verify` for the content/sibling oracle before trusting)");
}

fn dir_parts(dir: &str) -> Vec<Part> {
    vec![Part::Lit(dir.to_string()), Part::Slot(0), Part::Lit("_".into()), Part::Slot(1), Part::Lit(".".into()), Part::Slot(2)]
}

fn packed_bytes(strs: &[String]) -> (Vec<u32>, Vec<u32>) {
    let (mut bytes, mut off) = (Vec::new(), Vec::with_capacity(strs.len() * 2));
    for s in strs { off.push(bytes.len() as u32); off.push(s.len() as u32); bytes.extend(s.bytes().map(|b| b as u32)); }
    (bytes, off)
}

/// Persistent device + pipeline + the fixed vocabulary/target buffers. Only the directory literal changes
/// per `run_dir`, so the (large) vocab and target buffers upload ONCE and are reused across thousands of dirs.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    b_slots: wgpu::Buffer,
    b_tokoff: wgpu::Buffer,
    b_vbytes: wgpu::Buffer,
    b_targets: wgpu::Buffer,
    b_outcount: wgpu::Buffer,
    b_outids: wgpu::Buffer,
    b_meta: wgpu::Buffer,
    ntarget: u32,
    out_cap: u32,
}

impl Gpu {
    fn new(targets: &[u32], tok: &[String], ext: &[String]) -> Gpu {
        // token pool = tok ++ ext; slot0 & slot1 reuse the tok range, slot2 = ext range
        let pool: Vec<String> = tok.iter().chain(ext.iter()).cloned().collect();
        let (vbytes, tokoff) = packed_bytes(&pool);
        let slots: Vec<u32> = vec![0, tok.len() as u32, 0, tok.len() as u32, tok.len() as u32, ext.len() as u32];

        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default()
        })).expect("no GPU adapter");
        eprintln!("GPU: {}", adapter.get_info().name);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None, required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(), memory_hints: wgpu::MemoryHints::Performance,
        }, None)).expect("device");

        use wgpu::BufferUsages as U;
        let buf = |data: &[u32], usage| device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(data), usage,
        });
        let out_cap: u32 = 1 << 20;
        let b_slots = buf(&slots, U::STORAGE);
        let b_tokoff = buf(&tokoff, U::STORAGE);
        let b_vbytes = buf(&vbytes, U::STORAGE);
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
        Gpu { device, queue, pipeline, bgl, b_slots, b_tokoff, b_vbytes, b_targets, b_outcount, b_outids, b_meta, ntarget: targets.len() as u32, out_cap }
    }

    /// Enumerate `<dir>/<tok>_<tok>.<ext>` over the fixed vocabulary; return matching combination indices.
    fn run_dir(&self, dir: &str, total: u32) -> Vec<u32> {
        use wgpu::BufferUsages as U;
        // per-dir literals: dir, "_", "." packed together; parts reference them
        let mut lit: Vec<u32> = dir.bytes().map(|b| b as u32).collect();
        let dl = dir.len() as u32;
        lit.push('_' as u32); lit.push('.' as u32);
        let parts: Vec<u32> = vec![
            0, 0, dl, 0,        // Lit dir
            1, 0, 0, 0,         // Slot0 (tok)
            0, dl, 1, 0,        // Lit "_"
            1, 1, 0, 0,         // Slot1 (tok)
            0, dl + 1, 1, 0,    // Lit "."
            1, 2, 0, 0,         // Slot2 (ext)
        ];
        let b_lit = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&lit), usage: U::STORAGE });
        let b_parts = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&parts), usage: U::STORAGE });
        self.queue.write_buffer(&self.b_outcount, 0, &0u32.to_le_bytes());

        let bufs = [&self.b_meta, &b_parts, &self.b_slots, &self.b_tokoff, &self.b_vbytes, &b_lit, &self.b_targets, &self.b_outcount, &self.b_outids];
        let entries: Vec<wgpu::BindGroupEntry> = bufs.iter().enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry { binding: i as u32, resource: b.as_entire_binding() }).collect();
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &self.bgl, entries: &entries });

        let chunk: u32 = 65535 * 64;
        let mut base = 0u32;
        while base < total {
            let this = (total - base).min(chunk);
            let meta = Meta { num_parts: 6, num_slots: 3, base, total, out_cap: self.out_cap, ntarget: self.ntarget, _p0: 0, _p1: 0 };
            self.queue.write_buffer(&self.b_meta, 0, bytemuck::bytes_of(&meta));
            let mut enc = self.device.create_command_encoder(&Default::default());
            {
                let mut cp = enc.begin_compute_pass(&Default::default());
                cp.set_pipeline(&self.pipeline);
                cp.set_bind_group(0, &bind, &[]);
                cp.dispatch_workgroups((this + 63) / 64, 1, 1);
            }
            self.queue.submit([enc.finish()]);
            self.device.poll(wgpu::Maintain::Wait);
            base += this;
        }
        let read = |src: &wgpu::Buffer, n: u64| -> Vec<u32> {
            let staging = self.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: n * 4, usage: U::COPY_DST | U::MAP_READ, mapped_at_creation: false });
            let mut enc = self.device.create_command_encoder(&Default::default());
            enc.copy_buffer_to_buffer(src, 0, &staging, 0, n * 4);
            self.queue.submit([enc.finish()]);
            staging.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            self.device.poll(wgpu::Maintain::Wait);
            let data = staging.slice(..).get_mapped_range();
            let v = bytemuck::cast_slice(&data).to_vec();
            drop(data); staging.unmap(); v
        };
        let count = read(&self.b_outcount, 1)[0];
        let n = count.min(self.out_cap) as u64;
        if n == 0 { return Vec::new(); }
        read(&self.b_outids, n)
    }
}
