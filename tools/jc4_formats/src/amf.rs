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
    pub indices: Vec<u32>,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
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

    let (mut positions, mut indices) = (Vec::new(), Vec::new());
    for mesh in group.get("Meshes").and_then(|x| x.as_array()).unwrap_or(&empty) {
        let base = positions.len() as u32;
        let (p, idx) = decode_one_mesh(mesh, vb, ib)?;
        positions.extend(p);
        indices.extend(idx.into_iter().map(|i| i + base));
    }
    if positions.is_empty() { return Err("no vertices decoded".into()); }

    let bb = header.get("BoundingBox");
    let (bmin, bmax) = (arr3(bb.and_then(|b| b.get("Min"))), arr3(bb.and_then(|b| b.get("Max"))));
    // ORACLE: every decoded position must lie within the header BoundingBox (small epsilon)
    let eps = 1e-3 + 0.02 * (0..3).map(|i| (bmax[i] - bmin[i]).abs()).fold(0.0f32, f32::max);
    let inside = positions.iter().all(|p| (0..3).all(|i| p[i] >= bmin[i] - eps && p[i] <= bmax[i] + eps));
    if !inside { return Err("decoded positions fall outside the mesh BoundingBox (bad dequant/layout)".into()); }
    Ok(Mesh { positions, indices, bbox_min: bmin, bbox_max: bmax })
}

fn decode_one_mesh(mesh: &Value, vb: &[Vec<u8>], ib: &[Vec<u8>]) -> Result<(Vec<[f32; 3]>, Vec<u32>), String> {
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
    Ok((positions, indices))
}

/// Wavefront OBJ (positions + triangles) for eyeballing the decode in any 3D viewer.
pub fn to_obj(m: &Mesh) -> String {
    let mut s = String::with_capacity(m.positions.len() * 24 + m.indices.len() * 12);
    s.push_str("# jc4 amf mesh\n");
    for p in &m.positions { s.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2])); }
    for t in m.indices.chunks_exact(3) { s.push_str(&format!("f {} {} {}\n", t[0] + 1, t[1] + 1, t[2] + 1)); }
    s
}
