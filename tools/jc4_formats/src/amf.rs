//! AMF geometry decode — turn a `.meshc`/`.hrmeshc` (Avalanche Model Format, ADF-wrapped) into raw
//! positions + indices for rendering. The mesh header's `StreamAttributes[]` describe the interleaved
//! vertex layout: for each attribute, `Usage` (Position/Normal/TexCoord/…), `Format`, `StreamIndex`
//! (which vertex buffer/stride), `StreamOffset` (within the vertex), and `PackingData` (dequant). We
//! walk the Position stream at (VertexStreamOffsets[si] + i*stride + StreamOffset), dequantize per format
//! (SNORM positions rescale by the PackingData float), and read u16/u32 indices. Mirrors DECA
//! `ff_adf_amf_gltf`. Positions are validated against the header BoundingBox (see decode_mesh). See
//! [[model-amf]] and ../../docs/formats/adf.md (the container).
use serde_json::Value;
use crate::adf;

pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub normals: Vec<[f32; 3]>,     // smooth per-vertex normals (computed from geometry)
    pub indices: Vec<u32>,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub submeshes: Vec<SubMesh>,
}

/// A contiguous run of triangles sharing one material (`material` = the mesh's `SubMeshId`, matching a
/// modelc material Name). The three CarPaint texture PATHS are resolved to pixels by the caller.
pub struct SubMesh {
    pub tri_start: usize,
    pub tri_count: usize,
    pub material: Option<String>,
    pub diffuse: Option<String>, // _dif  (albedo)
    pub normal: Option<String>,  // _nrm  (BC5 tangent-space normal)
    pub mpm: Option<String>,     // _mpm  (metalness/roughness properties)
}

/// A decoded RGBA texture the rasterizer can sample.
pub struct Texture { pub w: usize, pub h: usize, pub rgba: Vec<u8> }
impl Texture {
    fn sample(&self, u: f32, v: f32) -> [u8; 4] {
        let wrap = |x: f32| { let f = x - x.floor(); (f * self.w.max(1) as f32) as usize % self.w.max(1) };
        let (px, py) = (wrap(u).min(self.w.saturating_sub(1)), {
            let f = v - v.floor(); ((f * self.h.max(1) as f32) as usize % self.h.max(1)).min(self.h.saturating_sub(1))
        });
        let o = (py * self.w + px) * 4;
        if o + 4 <= self.rgba.len() { [self.rgba[o], self.rgba[o + 1], self.rgba[o + 2], self.rgba[o + 3]] } else { [255, 255, 255, 255] }
    }
}

fn bytes_of(v: Option<&Value>) -> Vec<u8> {
    v.and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|n| n.as_u64().map(|u| u as u8)).collect())
        .unwrap_or_default()
}
fn u64_at(v: &Value, k: &str) -> u64 { v.get(k).and_then(|x| x.as_u64()).unwrap_or(0) }
fn arr3(v: Option<&Value>) -> [f32; 3] {
    let a = v.and_then(|x| x.as_array());
    let g = |i: usize| a.and_then(|a| a.get(i)).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    [g(0), g(1), g(2)]
}
fn u32s(v: &Value, k: &str) -> Vec<u64> {
    v.get(k).and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|n| n.as_u64()).collect()).unwrap_or_default()
}

fn i16le(d: &[u8], o: usize) -> i16 { i16::from_le_bytes([d[o], d[o + 1]]) }
fn f32le(d: &[u8], o: usize) -> f32 { f32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]) }

/// Vertex/index buffer byte arrays from an AmfMeshBuffers instance (`buffers` in a meshc or hrmeshc).
fn buffers_of(v: &Value) -> Result<(Vec<Vec<u8>>, Vec<Vec<u8>>), String> {
    let b = v.get("buffers").ok_or("no buffers instance")?;
    let vb = b.get("VertexBuffers").and_then(|x| x.as_array()).ok_or("no VertexBuffers")?
        .iter().map(|b| bytes_of(b.get("Data"))).collect();
    let ib = b.get("IndexBuffers").and_then(|x| x.as_array()).ok_or("no IndexBuffers")?
        .iter().map(|b| bytes_of(b.get("Data"))).collect();
    Ok((vb, ib))
}

/// Decode the highest-detail LOD present inline in a meshc (low LOD only).
pub fn decode_mesh(meshc: &[u8]) -> Result<Mesh, String> {
    let v = adf::parse(meshc.to_vec())?.decode_instances();
    let header = v.get("header").ok_or("meshc: no header instance")?.clone();
    let (vb, ib) = buffers_of(&v)?;
    decode_from(&header, &vb, &ib)
}

/// Decode the HIGH LOD: the meshc supplies the header (StreamAttributes/LodGroups); the hrmeshc supplies
/// the high-detail buffers. LodGroup buffer indices address the combined list `meshc ++ hrmeshc`.
pub fn decode_mesh_hr(meshc: &[u8], hrmeshc: &[u8]) -> Result<Mesh, String> {
    let mv = adf::parse(meshc.to_vec())?.decode_instances();
    let header = mv.get("header").ok_or("meshc: no header instance")?.clone();
    let (mut vb, mut ib) = buffers_of(&mv)?;
    let hv = adf::parse(hrmeshc.to_vec())?.decode_instances();
    let (hvb, hib) = buffers_of(&hv)?;
    vb.extend(hvb);
    ib.extend(hib);
    decode_from(&header, &vb, &ib)
}

fn decode_from(header: &Value, vb: &[Vec<u8>], ib: &[Vec<u8>]) -> Result<Mesh, String> {
    let groups = header.get("LodGroups").and_then(|x| x.as_array()).ok_or("no LodGroups")?;
    let empty = Vec::new();
    let meshes_of = |g: &Value| g.get("Meshes").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    // A meshc inlines only the low-detail LOD's buffers (higher LODs reference the .hrmeshc). Pick the
    // most-detailed LOD group whose referenced buffers are all present in THIS file.
    let inline = |g: &Value| {
        let ms = meshes_of(g);
        !ms.is_empty() && ms.iter().all(|m| {
            (u64_at(m, "IndexBufferIndex") as usize) < ib.len()
                && u32s(m, "VertexBufferIndices").iter().all(|&x| (x as usize) < vb.len())
        })
    };
    let group = groups.iter().filter(|g| inline(g))
        .max_by_key(|g| meshes_of(g).iter().map(|m| u64_at(m, "VertexCount")).sum::<u64>())
        .ok_or("no inline-decodable LOD group (buffers may be in the .hrmeshc)")?;

    let (mut positions, mut uvs, mut indices, mut submeshes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for mesh in group.get("Meshes").and_then(|x| x.as_array()).unwrap_or(&empty) {
        let base = positions.len() as u32;
        let tri_base = indices.len() / 3;
        let (p, uv, idx, subs) = decode_one_mesh(mesh, vb, ib)?;
        positions.extend(p);
        uvs.extend(uv);
        indices.extend(idx.into_iter().map(|i| i + base));
        for (ts, tc, id) in subs {
            submeshes.push(SubMesh { tri_start: tri_base + ts, tri_count: tc,
                material: Some(id).filter(|s| !s.is_empty()), diffuse: None, normal: None, mpm: None });
        }
    }
    if positions.is_empty() { return Err("no vertices decoded".into()); }

    let bb = header.get("BoundingBox");
    let (bmin, bmax) = (arr3(bb.and_then(|b| b.get("Min"))), arr3(bb.and_then(|b| b.get("Max"))));
    // ORACLE: every decoded position must lie within the header BoundingBox (small epsilon)
    let eps = 1e-3 + 0.02 * (0..3).map(|i| (bmax[i] - bmin[i]).abs()).fold(0.0f32, f32::max);
    let inside = positions.iter().all(|p| (0..3).all(|i| p[i] >= bmin[i] - eps && p[i] <= bmax[i] + eps));
    if !inside { return Err("decoded positions fall outside the mesh BoundingBox (bad dequant/layout)".into()); }
    let normals = smooth_normals(&positions, &indices);
    Ok(Mesh { positions, uvs, normals, indices, bbox_min: bmin, bbox_max: bmax, submeshes })
}

/// Area-weighted smooth per-vertex normals from the geometry (no packed-normal-stream decode needed).
fn smooth_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut n = vec![[0f32; 3]; positions.len()];
    for t in indices.chunks_exact(3) {
        let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let (pa, pb, pc) = (positions[a], positions[b], positions[c]);
        let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
        let w = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
        let f = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]]; // area-weighted
        for &vi in &[a, b, c] { for i in 0..3 { n[vi][i] += f[i]; } }
    }
    for v in &mut n {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if l > 1e-9 { for i in 0..3 { v[i] /= l; } } else { *v = [0.0, 1.0, 0.0]; }
    }
    n
}

fn decode_one_mesh(mesh: &Value, vb: &[Vec<u8>], ib: &[Vec<u8>]) -> Result<(Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>, Vec<(usize, usize, String)>), String> {
    let vcount = u64_at(mesh, "VertexCount") as usize;
    let attrs = mesh.get("StreamAttributes").and_then(|x| x.as_array()).ok_or("no StreamAttributes")?;
    let pos = attrs.iter().find(|a| a.get("Usage").and_then(|u| u.as_str()) == Some("AmfUsage_Position"))
        .ok_or("no Position attribute")?;

    let si = u64_at(pos, "StreamIndex") as usize;
    let vbi = *u32s(mesh, "VertexBufferIndices").get(si).ok_or("StreamIndex oob")? as usize;
    let voff = *u32s(mesh, "VertexStreamOffsets").get(si).unwrap_or(&0) as usize;
    let stride = u64_at(pos, "StreamStride") as usize;
    let soff = u64_at(pos, "StreamOffset") as usize;
    let fmt = pos.get("Format").and_then(|f| f.as_str()).unwrap_or("");
    let pack = bytes_of(pos.get("PackingData"));
    let scale = if pack.len() >= 4 { f32le(&pack, 0) } else { 1.0 };
    let data = vb.get(vbi).ok_or("vertex buffer index oob")?;

    let mut positions = Vec::with_capacity(vcount);
    for i in 0..vcount {
        let o = voff + i * stride + soff;
        let p = match fmt {
            "AmfFormat_R16G16B16_SNORM" | "AmfFormat_R16G16B16A16_SNORM" => {
                if o + 6 > data.len() { return Err("position stream overruns vertex buffer".into()); }
                let s = scale / 32767.0;
                [i16le(data, o) as f32 * s, i16le(data, o + 2) as f32 * s, i16le(data, o + 4) as f32 * s]
            }
            "AmfFormat_R32G32B32_FLOAT" => {
                if o + 12 > data.len() { return Err("position stream overruns vertex buffer".into()); }
                [f32le(data, o), f32le(data, o + 4), f32le(data, o + 8)]
            }
            other => return Err(format!("position format {other} not handled yet")),
        };
        positions.push(p);
    }

    // UVs (first TextureCoordinate stream); (0,0) if absent
    let mut uvs = vec![[0f32, 0f32]; vcount];
    if let Some(uv) = attrs.iter().find(|a| a.get("Usage").and_then(|u| u.as_str()) == Some("AmfUsage_TextureCoordinate")) {
        let si = u64_at(uv, "StreamIndex") as usize;
        let vbi = *u32s(mesh, "VertexBufferIndices").get(si).unwrap_or(&0) as usize;
        let voff = *u32s(mesh, "VertexStreamOffsets").get(si).unwrap_or(&0) as usize;
        let stride = u64_at(uv, "StreamStride") as usize;
        let soff = u64_at(uv, "StreamOffset") as usize;
        let fmt = uv.get("Format").and_then(|f| f.as_str()).unwrap_or("");
        let pack = bytes_of(uv.get("PackingData"));
        let scale = if pack.len() >= 4 { f32le(&pack, 0) } else { 1.0 };
        if let Some(data) = vb.get(vbi) {
            for i in 0..vcount {
                let o = voff + i * stride + soff;
                uvs[i] = match fmt {
                    "AmfFormat_R16G16_SNORM" if o + 4 <= data.len() => { let s = scale / 32767.0; [i16le(data, o) as f32 * s, i16le(data, o + 2) as f32 * s] }
                    "AmfFormat_R16G16_UNORM" if o + 4 <= data.len() => { let s = scale / 65535.0; [u16::from_le_bytes([data[o], data[o + 1]]) as f32 * s, u16::from_le_bytes([data[o + 2], data[o + 3]]) as f32 * s] }
                    "AmfFormat_R32G32_FLOAT" if o + 8 <= data.len() => [f32le(data, o), f32le(data, o + 4)],
                    _ => [0.0, 0.0],
                };
            }
        }
    }

    let ibi = u64_at(mesh, "IndexBufferIndex") as usize;
    let ibo = u64_at(mesh, "IndexBufferOffset") as usize;
    let istride = u64_at(mesh, "IndexBufferStride").max(2) as usize;
    let icount = u64_at(mesh, "IndexCount") as usize;
    let idata = ib.get(ibi).ok_or("index buffer index oob")?;
    let mut indices = Vec::with_capacity(icount);
    for j in 0..icount {
        let o = ibo + j * istride;
        if o + istride > idata.len() { return Err("index stream overruns index buffer".into()); }
        let idx = if istride == 4 { u32::from_le_bytes([idata[o], idata[o + 1], idata[o + 2], idata[o + 3]]) }
                  else { u16::from_le_bytes([idata[o], idata[o + 1]]) as u32 };
        if idx as usize >= vcount { return Err(format!("index {idx} >= vertex count {vcount}")); }
        indices.push(idx);
    }

    // per-material submesh ranges (SubMeshId = material name; IndexStreamOffset/IndexCount in indices)
    let mut subs: Vec<(usize, usize, String)> = mesh.get("SubMeshes").and_then(|x| x.as_array())
        .map(|arr| arr.iter().map(|s| {
            let ts = u64_at(s, "IndexStreamOffset") as usize / 3;
            let tc = u64_at(s, "IndexCount") as usize / 3;
            let id = s.get("SubMeshId").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (ts, tc, id)
        }).collect()).unwrap_or_default();
    if subs.is_empty() { subs.push((0, indices.len() / 3, String::new())); }
    Ok((positions, uvs, indices, subs))
}

// ── headless z-buffered rasterizer (render oracle) ───────────────────────────────────────────────
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 { crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB88320 } else { crc >> 1 }; }
    }
    !crc
}
/// Minimal RGB PNG (filter 0, zlib via flate2) so a render can be viewed directly.
fn png(w: usize, h: usize, rgb: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut raw = Vec::with_capacity(h * (1 + w * 3));
    for y in 0..h { raw.push(0); raw.extend_from_slice(&rgb[y * w * 3..(y + 1) * w * 3]); }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    z.write_all(&raw).unwrap();
    let idat = z.finish().unwrap();
    let mut out = vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |typ: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut cd = typ.to_vec(); cd.extend_from_slice(data);
        out.extend_from_slice(&cd);
        out.extend_from_slice(&crc32(&cd).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit, RGB
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &idat);
    chunk(b"IEND", &[]);
    out
}

/// Per-submesh CarPaint texture indices into the shared `Texture` pool.
#[derive(Clone, Copy, Default)]
pub struct SubTex { pub dif: Option<usize>, pub nrm: Option<usize>, pub mpm: Option<usize> }

/// Software-rasterize the mesh to RGBA (transparent bg, per-pixel z-buffer) with a CarPaint-ish shader:
/// interpolated smooth normals, albedo from `_dif`, metalness+roughness from `_mpm` (→ metal kills diffuse
/// and tints the specular; dielectric gets a small white spec), Blinn-Phong specular, ambient + a faint
/// metal env term. `nrm` maps not yet applied (needs tangents). Untextured falls back to the `base` tint.
pub fn rasterize_rgba(m: &Mesh, w: usize, h: usize, yaw: f32, pitch: f32, base: [u8; 3], paint: [f32; 3], pool: &[Texture], sub_tex: &[SubTex]) -> Vec<u8> {
    let mut color = vec![0u8; w * h * 4];
    let mut zbuf = vec![f32::NEG_INFINITY; w * h];
    let (cy, sy, cp, sp) = (yaw.cos(), yaw.sin(), pitch.cos(), pitch.sin());
    let rot = |v: [f32; 3]| { let (x, z) = (v[0] * cy + v[2] * sy, -v[0] * sy + v[2] * cy); [x, v[1] * cp - z * sp, v[1] * sp + z * cp] };
    let c = [0, 1, 2].map(|i| (m.bbox_min[i] + m.bbox_max[i]) * 0.5);
    let radius = (0..3).map(|i| (m.bbox_max[i] - m.bbox_min[i]).abs()).fold(1e-6f32, f32::max) * 0.5;
    let scale = w.min(h) as f32 / (2.0 * radius) * 0.8;
    let proj: Vec<[f32; 3]> = m.positions.iter().map(|v| {
        let r = rot([v[0] - c[0], v[1] - c[1], v[2] - c[2]]);
        [w as f32 * 0.5 + r[0] * scale, h as f32 * 0.5 - r[1] * scale, r[2]]
    }).collect();
    let rn: Vec<[f32; 3]> = m.normals.iter().map(|n| rot(*n)).collect(); // view-space smooth normals
    let tri_count = m.indices.len() / 3;
    let get = |idx: Option<usize>| idx.and_then(|i| pool.get(i));
    let (mut tri_dif, mut tri_mpm): (Vec<Option<&Texture>>, Vec<Option<&Texture>>) = (vec![None; tri_count], vec![None; tri_count]);
    for (si, sm) in m.submeshes.iter().enumerate() {
        let st = sub_tex.get(si).copied().unwrap_or_default();
        let (d, mp) = (get(st.dif), get(st.mpm));
        for k in sm.tri_start..(sm.tri_start + sm.tri_count).min(tri_count) { tri_dif[k] = d; tri_mpm[k] = mp; }
    }
    let uv = |i: usize| m.uvs.get(i).copied().unwrap_or([0.0, 0.0]);
    let norm = |v: [f32; 3]| { let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9); [v[0] / l, v[1] / l, v[2] / l] };
    let light = norm([0.4, 0.55, 0.75]);
    let view = [0.0, 0.0, 1.0];
    let half = norm([light[0] + view[0], light[1] + view[1], light[2] + view[2]]);
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let edge = |ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32| (bx - ax) * (py - ay) - (by - ay) * (px - ax);
    for (ti, t) in m.indices.chunks_exact(3).enumerate() {
        let (ia, ib, ic) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let (pa, pb, pc) = (proj[ia], proj[ib], proj[ic]);
        let (ua, ub, uc) = (uv(ia), uv(ib), uv(ic));
        let (na, nb, nc) = (rn.get(ia).copied().unwrap_or([0.0, 0.0, 1.0]), rn.get(ib).copied().unwrap_or([0.0, 0.0, 1.0]), rn.get(ic).copied().unwrap_or([0.0, 0.0, 1.0]));
        let (dif, mpm) = (tri_dif[ti], tri_mpm[ti]);
        let area = edge(pa[0], pa[1], pb[0], pb[1], pc[0], pc[1]);
        if area.abs() < 1e-6 { continue; }
        let (minx, maxx) = (pa[0].min(pb[0]).min(pc[0]).floor().max(0.0) as usize, pa[0].max(pb[0]).max(pc[0]).ceil().min(w as f32 - 1.0) as usize);
        let (miny, maxy) = (pa[1].min(pb[1]).min(pc[1]).floor().max(0.0) as usize, pa[1].max(pb[1]).max(pc[1]).ceil().min(h as f32 - 1.0) as usize);
        for py in miny..=maxy {
            for px in minx..=maxx {
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let (w0, w1, w2) = (edge(pb[0], pb[1], pc[0], pc[1], fx, fy) / area,
                                    edge(pc[0], pc[1], pa[0], pa[1], fx, fy) / area,
                                    edge(pa[0], pa[1], pb[0], pb[1], fx, fy) / area);
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 { continue; }
                let depth = w0 * pa[2] + w1 * pb[2] + w2 * pc[2];
                let i = py * w + px;
                if depth <= zbuf[i] { continue; }
                let (uu, vv) = (w0 * ua[0] + w1 * ub[0] + w2 * uc[0], w0 * ua[1] + w1 * ub[1] + w2 * uc[1]);
                // albedo
                let alb0 = match dif {
                    Some(tx) => { let s = tx.sample(uu, vv); if s[3] < 96 { continue; } [s[0] as f32 / 255.0, s[1] as f32 / 255.0, s[2] as f32 / 255.0] }
                    None => [base[0] as f32 / 255.0, base[1] as f32 / 255.0, base[2] as f32 / 255.0],
                };
                // metalness / roughness from _mpm (R=metal, G=rough) — defaults: metallic body
                let (metal, rough) = match mpm { Some(tx) => { let s = tx.sample(uu, vv); (s[0] as f32 / 255.0, (s[1] as f32 / 255.0).max(0.08)) } None => (0.85, 0.4) };
                // paint tint: modulate the base color by the chosen paint, scaled by metalness so the
                // painted metal body takes the colour while rubber/glass (non-metal) stay mostly neutral
                let alb = [0, 1, 2].map(|ch| alb0[ch] * (paint[ch] * metal + (1.0 - metal)));
                let n = norm([w0 * na[0] + w1 * nb[0] + w2 * nc[0], w0 * na[1] + w1 * nb[1] + w2 * nc[1], w0 * na[2] + w1 * nb[2] + w2 * nc[2]]);
                let ndl = dot(n, light).max(0.0);
                let ndh = dot(n, half).max(0.0);
                // diffuse (metal has none) + ambient
                let kd = (1.0 - metal) * (ndl * 0.85 + 0.28);
                // specular: F0 from albedo (metal) or dielectric reflectance; sharpness from gloss
                let gloss = 1.0 - rough;
                let shin = 6.0 + gloss * gloss * 220.0;
                let specpow = if ndl > 0.0 { ndh.powf(shin) } else { 0.0 };
                let dielec = 0.59 * 0.08;
                let env = 0.28 * metal; // faint reflected ambient so metal isn't black
                let mut out = [0u8; 4];
                for ch in 0..3 {
                    let f0 = dielec * (1.0 - metal) + alb[ch] * metal;
                    let v = alb[ch] * kd + specpow * (f0 + (1.0 - metal) * 0.35) + alb[ch] * env;
                    out[ch] = (v.clamp(0.0, 1.0) * 255.0) as u8;
                }
                out[3] = 255;
                zbuf[i] = depth;
                color[i * 4..i * 4 + 4].copy_from_slice(&out);
            }
        }
    }
    color
}

/// Encode an RGBA buffer as an RGB PNG (alpha over black) — for dumping a decoded texture to view.
pub fn to_png_rgba(w: usize, h: usize, rgba: &[u8]) -> Vec<u8> {
    let mut rgb = vec![0u8; w * h * 3];
    for (o, px) in rgba.chunks_exact(4).enumerate() { rgb[o * 3..o * 3 + 3].copy_from_slice(&px[..3]); }
    png(w, h, &rgb)
}

/// Render oracle: rasterize and composite over a dark ground → RGB PNG (`jc4_arc mesh/model --png`).
pub fn render_png(m: &Mesh, size: usize, yaw: f32, pitch: f32) -> Vec<u8> { render_png_tex(m, size, yaw, pitch, [1.0; 3], &[], &[]) }
pub fn render_png_tex(m: &Mesh, size: usize, yaw: f32, pitch: f32, paint: [f32; 3], pool: &[Texture], sub_tex: &[SubTex]) -> Vec<u8> {
    let (w, h) = (size, size);
    let rgba = rasterize_rgba(m, w, h, yaw, pitch, [200, 200, 205], paint, pool, sub_tex);
    let bg = [16u8, 22, 28];
    let mut rgb = vec![0u8; w * h * 3];
    for (o, px) in rgba.chunks_exact(4).enumerate() {
        let a = px[3] as u32;
        for c in 0..3 { rgb[o * 3 + c] = ((px[c] as u32 * a + bg[c] as u32 * (255 - a)) / 255) as u8; }
    }
    png(w, h, &rgb)
}

/// Destruction/variant parts that should NOT render on the intact model (debris fragments, destroyed and
/// broken states). Gated out of `decode_model`. Mesh-swap (`mshswap`) alternates are kept — they overlap
/// the base in place, so they're harmless (a later refinement could pick one skin).
fn is_render_part(base: &str) -> bool {
    let b = base.to_ascii_lowercase();
    !(b.contains("_debris") || b.contains("_dst") || b.contains("_destroyed") || b.contains("_broken") || b.contains("_wreck"))
}

const IDENT: [f32; 16] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];
/// Apply a row-major mat4 (translation in [12..15]) to a point (row-vector convention v·M).
fn xform(m: &[f32; 16], v: [f32; 3]) -> [f32; 3] {
    [v[0] * m[0] + v[1] * m[4] + v[2] * m[8] + m[12],
     v[0] * m[1] + v[1] * m[5] + v[2] * m[9] + m[13],
     v[0] * m[2] + v[1] * m[6] + v[2] * m[10] + m[14]]
}
/// Match a SARC part base name to a declared render part, tolerating the `mshswap` mesh-swap suffix.
fn part_xform(base: &str, render: &std::collections::HashMap<String, [f32; 16]>) -> Option<[f32; 16]> {
    let cands = [base.to_string(), base.trim_end_matches("mshswap").to_string(), base.trim_end_matches("_mshswap").to_string()];
    cands.iter().find_map(|c| render.get(c).copied())
}

/// Which layers of a model to assemble. `render` = the entity's declared render parts (the vehicle);
/// `other` = bundled parts the blueprint does NOT render (e.g. a spawned payload); `debris` = destruction
/// fragments. Faithful default: render only.
#[derive(Clone, Copy)]
pub struct PartSel { pub render: bool, pub other: bool, pub debris: bool }
impl Default for PartSel { fn default() -> Self { PartSel { render: true, other: false, debris: false } } }

#[derive(PartialEq)]
pub enum PartClass { Render, Other, Debris }
fn classify(base: &str, render: &Option<std::collections::HashMap<String, [f32; 16]>>) -> (PartClass, [f32; 16]) {
    if !is_render_part(base) { return (PartClass::Debris, IDENT); }
    match render {
        Some(map) => match part_xform(base, map) { Some(t) => (PartClass::Render, t), None => (PartClass::Other, IDENT) },
        None => (PartClass::Render, IDENT),
    }
}

/// Count mesh parts per class `(render, other, debris)` — for the layer toggles.
pub fn model_part_counts(sarc_bytes: &[u8], rtpc_bytes: Option<&[u8]>) -> (usize, usize, usize) {
    let render = rtpc_bytes.and_then(|r| crate::rtpc::render_parts(r).ok()).map(|v| v.into_iter().collect());
    let (mut a, mut b, mut c) = (0, 0, 0);
    if let Ok(members) = crate::sarc::parse(sarc_bytes) {
        for m in &members {
            if !(m.stored && m.name.ends_with(".meshc")) { continue; }
            let base = m.name.trim_end_matches(".meshc").rsplit('/').next().unwrap_or("").to_ascii_lowercase();
            match classify(&base, &render).0 { PartClass::Render => a += 1, PartClass::Other => b += 1, PartClass::Debris => c += 1 }
        }
    }
    (a, b, c)
}

pub fn decode_model(sarc_bytes: &[u8]) -> Result<Mesh, String> { decode_model_asm(sarc_bytes, None, PartSel::default()) }

/// FAITHFUL model assembly. With the entity `.epe` RTPC, render the parts the entity declares (`sel.render`),
/// each placed by its `world` transform; optionally the bundled payload (`sel.other`) and debris (`sel.debris`).
/// Without the RTPC, all non-debris parts are treated as render. Each `.meshc` is hr-paired for high LOD.
pub fn decode_model_asm(sarc_bytes: &[u8], rtpc_bytes: Option<&[u8]>, sel: PartSel) -> Result<Mesh, String> {
    let members = crate::sarc::parse(sarc_bytes)?;
    let render: Option<std::collections::HashMap<String, [f32; 16]>> = rtpc_bytes
        .and_then(|r| crate::rtpc::render_parts(r).ok())
        .map(|v| v.into_iter().collect());
    let hr: std::collections::HashMap<&str, &[u8]> = members.iter()
        .filter(|m| m.stored && m.name.ends_with(".hrmeshc"))
        .filter_map(|m| m.data(sarc_bytes).map(|d| (m.name.trim_end_matches(".hrmeshc"), d)))
        .collect();
    // sibling .modelc bytes by base name (for the diffuse texture path)
    let modelc: std::collections::HashMap<&str, &[u8]> = members.iter()
        .filter(|m| m.stored && m.name.ends_with(".modelc"))
        .filter_map(|m| m.data(sarc_bytes).map(|d| (m.name.trim_end_matches(".modelc"), d)))
        .collect();
    let mut out = Mesh { positions: Vec::new(), uvs: Vec::new(), normals: Vec::new(), indices: Vec::new(),
        bbox_min: [f32::MAX; 3], bbox_max: [f32::MIN; 3], submeshes: Vec::new() };
    let mut parts = 0;
    for m in &members {
        if !(m.stored && m.name.ends_with(".meshc")) { continue; }
        let stem = m.name.trim_end_matches(".meshc");
        let base = stem.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
        let (class, tf) = classify(&base, &render);
        let want = match class { PartClass::Render => sel.render, PartClass::Other => sel.other, PartClass::Debris => sel.debris };
        if !want { continue; }
        let d = match m.data(sarc_bytes) { Some(d) => d, None => continue };
        let mesh = match hr.get(stem) { Some(h) => decode_mesh_hr(d, h), None => decode_mesh(d) };
        let mesh = match mesh { Ok(x) => x, Err(_) => continue };
        let base_i = out.positions.len() as u32;
        let tri_start = out.indices.len() / 3;
        let identity = tf == IDENT;
        for &p in &mesh.positions {
            let q = if identity { p } else { xform(&tf, p) };
            out.positions.push(q);
            for i in 0..3 { out.bbox_min[i] = out.bbox_min[i].min(q[i]); out.bbox_max[i] = out.bbox_max[i].max(q[i]); }
        }
        for &nrm in &mesh.normals { out.normals.push(if identity { nrm } else { xform_normal(&tf, nrm) }); }
        out.uvs.extend(&mesh.uvs);
        out.indices.extend(mesh.indices.iter().map(|i| i + base_i));
        let matmap = modelc.get(stem).map(|mc| material_textures_map(mc)).unwrap_or_default();
        for sm in &mesh.submeshes {
            let mt = sm.material.as_ref().and_then(|m| matmap.get(m));
            out.submeshes.push(SubMesh {
                tri_start: tri_start + sm.tri_start, tri_count: sm.tri_count, material: sm.material.clone(),
                diffuse: mt.and_then(|t| t.dif.clone()), normal: mt.and_then(|t| t.nrm.clone()), mpm: mt.and_then(|t| t.mpm.clone()),
            });
        }
        parts += 1;
    }
    if parts == 0 { return Err("no render mesh parts".into()); }
    Ok(out)
}

/// Rotate a normal by a transform's 3×3 (no translation), renormalized.
fn xform_normal(m: &[f32; 16], n: [f32; 3]) -> [f32; 3] {
    let r = [n[0] * m[0] + n[1] * m[4] + n[2] * m[8], n[0] * m[1] + n[1] * m[5] + n[2] * m[9], n[0] * m[2] + n[1] * m[6] + n[2] * m[10]];
    let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    if l > 1e-9 { [r[0] / l, r[1] / l, r[2] / l] } else { n }
}

/// A material's CarPaint texture set (paths).
struct MatTex { dif: Option<String>, nrm: Option<String>, mpm: Option<String> }
/// Map each `.modelc` material Name → its `_dif`/`_nrm`/`_mpm` texture paths. Submeshes bind by `SubMeshId`.
fn material_textures_map(bytes: &[u8]) -> std::collections::HashMap<String, MatTex> {
    let mut out = std::collections::HashMap::new();
    let v = match adf::parse(bytes.to_vec()) { Ok(a) => a.decode_instances(), Err(_) => return out };
    fn find_mats(v: &Value) -> Option<&Vec<Value>> {
        match v {
            Value::Object(m) => {
                if let Some(Value::Array(a)) = m.get("Materials") { if a.iter().any(|x| x.get("Name").is_some()) { return Some(a); } }
                m.values().find_map(find_mats)
            }
            Value::Array(a) => a.iter().find_map(find_mats),
            _ => None,
        }
    }
    if let Some(mats) = find_mats(&v) {
        for mat in mats {
            let name = mat.get("Name").and_then(|n| n.as_str()).unwrap_or("");
            if name.is_empty() { continue; }
            let texs: Vec<String> = mat.get("Textures").and_then(|t| t.as_array())
                .map(|a| a.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
            let pick = |keys: &[&str]| texs.iter().find(|s| {
                let l = s.to_ascii_lowercase();
                l.ends_with(".ddsc") && !l.contains("dummy") && keys.iter().any(|k| l.contains(k))
            }).cloned();
            out.insert(name.to_string(), MatTex {
                dif: pick(&["_dif", "_diff", "albedo"]),
                nrm: pick(&["_nrm", "_normal"]),
                mpm: pick(&["_mpm"]),
            });
        }
    }
    out
}

/// Wavefront OBJ (positions + triangles) for eyeballing the decode in any 3D viewer.
pub fn to_obj(m: &Mesh) -> String {
    let mut s = String::with_capacity(m.positions.len() * 24 + m.indices.len() * 12);
    s.push_str("# jc4 amf mesh\n");
    for p in &m.positions { s.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2])); }
    for t in m.indices.chunks_exact(3) { s.push_str(&format!("f {} {} {}\n", t[0] + 1, t[1] + 1, t[2] + 1)); }
    s
}
