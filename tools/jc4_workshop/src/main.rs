// jc4_workshop — JC4 asset browser & inspector (wgpu + eframe). "Storm-rig" identity.
//
// The app frame mirrors the Mercs2/Sab workshops: command bar · left icon rail · navigator ·
// viewport · inspector · status bar, with Inspect + Settings pages. Navigator/inspector are wired to
// the real jc4_formats backends (tab/oodle/adf/avtx/sarc/bundle) — archives browse as composite UNITS
// (Models / Structures / Textures / Data), not flat files. The render viewport is intentionally a
// STUB (drawn placeholder); the wgpu paint pass slots into `viewport()` next.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, RichText, Stroke};
use jc4_formats::{adf, avtx, bundle, hash::hashlittle, oodle::Oodle, sarc, tab};

// ── storm-rig palette ──────────────────────────────────────────────────────────────────────────
const G0: Color32 = Color32::from_rgb(0x0b, 0x10, 0x15); // app ground
const G1: Color32 = Color32::from_rgb(0x0f, 0x16, 0x1c); // panels
const G2: Color32 = Color32::from_rgb(0x16, 0x1f, 0x27); // cards / inputs
const G3: Color32 = Color32::from_rgb(0x1e, 0x2a, 0x34); // raised / hover
const LINE: Color32 = Color32::from_rgb(0x24, 0x34, 0x40);
const LINE2: Color32 = Color32::from_rgb(0x37, 0x48, 0x5a);
const TX: Color32 = Color32::from_rgb(0xd6, 0xde, 0xe4);
const DIM: Color32 = Color32::from_rgb(0x8a, 0x97, 0xa3);
const FAINT: Color32 = Color32::from_rgb(0x58, 0x66, 0x74);
const ACC: Color32 = Color32::from_rgb(0xff, 0x7a, 0x33); // explosive orange — live/selected
const ACC_SOFT: Color32 = Color32::from_rgb(0x2a, 0x17, 0x10);
const VOLT: Color32 = Color32::from_rgb(0x3f, 0xd0, 0xd8); // lightning teal — verified/good
const INFO: Color32 = Color32::from_rgb(0x5a, 0xa0, 0xd8);

fn install_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(TX);
    v.panel_fill = G1;
    v.window_fill = G1;
    v.extreme_bg_color = G0;
    v.faint_bg_color = G2;
    v.hyperlink_color = ACC;
    v.selection.bg_fill = ACC_SOFT;
    v.selection.stroke = Stroke::new(1.0, ACC);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = G1;
    w.noninteractive.fg_stroke = Stroke::new(1.0, DIM);
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.inactive.bg_fill = G2;
    w.inactive.weak_bg_fill = G2;
    w.inactive.fg_stroke = Stroke::new(1.0, TX);
    w.inactive.bg_stroke = Stroke::new(1.0, LINE);
    w.hovered.bg_fill = G3;
    w.hovered.weak_bg_fill = G3;
    w.hovered.bg_stroke = Stroke::new(1.0, LINE2);
    w.active.bg_fill = G3;
    w.active.bg_stroke = Stroke::new(1.0, ACC);
    ctx.set_visuals(v);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(7.0, 6.0);
    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]).with_title("JC4 Workshop"),
        ..Default::default()
    };
    eframe::run_native("jc4_workshop", opts, Box::new(|cc| {
        install_theme(&cc.egui_ctx);
        let mut app = Workshop::default();
        app.oodle_dll = tab::default_oodle_dll();
        Ok(Box::new(app))
    }))
}

// ── unit taxonomy: classify an entry by its resolved path extension ──────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum Kind { Model, Structure, Entity, Texture, Ui, Data, Other }
impl Kind {
    fn of_path(p: &str) -> Kind {
        let e = p.rsplit('.').next().unwrap_or("");
        match e {
            "modelc" | "meshc" | "hrmeshc" | "mdic" => Kind::Model,
            "resourcebundle" => Kind::Structure,
            "ee" | "epe" | "epe_adf" => Kind::Entity,
            "ddsc" | "hmddsc" => Kind::Texture,
            "gfx" => Kind::Ui,
            "environc" | "adf" | "aisystunec" | "bin" => Kind::Data,
            _ => Kind::Other,
        }
    }
    fn tag(self) -> (&'static str, Color32) {
        match self {
            Kind::Model => ("MODEL", ACC), Kind::Structure => ("STRUCT", VOLT),
            Kind::Entity => ("ENTITY", INFO), Kind::Texture => ("TEX", VOLT),
            Kind::Ui => ("UI", INFO), Kind::Data => ("DATA", DIM), Kind::Other => ("···", FAINT),
        }
    }
}

// ── decoded inspector views ──────────────────────────────────────────────────────────────────────
struct ModelPart { name: String, lods: usize, lod_factor: f64, materials: Vec<Material>, mesh: Option<MeshStat> }
struct Material { name: String, render_block: String, textures: Vec<String> }
struct MeshStat { groups: usize, meshes: u64, verts: u64, indices: u64, hires: String }
struct BundleView { members: Vec<(u32, u32, usize, String, Vec<(String, String)>)> } // name_hash,type_hash,size,ext,fields
struct SarcView { stored: usize, refs: usize, members: Vec<(String, usize, bool)> }
struct TexView { info: String, dims: Option<(u32, u32, String)> }

enum Preview {
    Model { parts: Vec<ModelPart> },
    Structure(BundleView),
    Sarc(SarcView),
    Texture(TexView),
    Data(String),
    Raw(String),
    Error(String),
}

struct Archive { tab_path: PathBuf, arc_path: PathBuf, tab: tab::Tab }

#[derive(Default, PartialEq, Clone, Copy)]
enum Page { #[default] Inspect, Settings }

#[derive(Default)]
struct Workshop {
    page: Page,
    archive: Option<Archive>,
    names: BTreeMap<u32, String>,
    filter: String,
    selected: Option<usize>,
    sel_kind: Option<Kind>,
    preview: Option<Preview>,
    status: String,
    oodle_dll: String,
    oodle: Option<Oodle>,
}

fn basename(s: &str) -> &str { s.rsplit(['/', '\\']).next().unwrap_or(s) }
fn name_of(p: &Path) -> String { p.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string() }

impl Workshop {
    fn open_archive(&mut self, tab_path: PathBuf) {
        let arc_path = tab_path.with_extension("arc");
        match std::fs::read(&tab_path).map_err(|e| e.to_string()).and_then(|b| tab::parse_tab(&b)) {
            Ok(t) => {
                self.status = format!("loaded {} entries from {}", t.entries.len(), name_of(&tab_path));
                self.archive = Some(Archive { tab_path, arc_path, tab: t });
                self.selected = None; self.preview = None;
            }
            Err(e) => self.status = format!("open failed: {e}"),
        }
    }

    fn load_filelist(&mut self, path: PathBuf) {
        if let Ok(txt) = std::fs::read_to_string(&path) {
            for line in txt.lines() {
                let p = line.trim();
                if !p.is_empty() && !p.starts_with(';') { self.names.insert(hashlittle(p.as_bytes(), 0), p.to_string()); }
            }
            self.status = format!("{} known names", self.names.len());
        }
    }

    fn entry_kind(&self, e: &tab::Entry) -> Kind {
        self.names.get(&e.name_hash).map(|p| Kind::of_path(p)).unwrap_or(Kind::Other)
    }
    fn entry_label(&self, e: &tab::Entry) -> String {
        self.names.get(&e.name_hash).map(|p| basename(p).to_string()).unwrap_or_else(|| format!("{:08x}", e.name_hash))
    }

    fn select(&mut self, i: usize) {
        self.selected = Some(i);
        let arc = match &self.archive { Some(a) => a, None => return };
        let entry = arc.tab.entries[i];
        self.sel_kind = Some(self.entry_kind(&entry));
        let path_ext = self.names.get(&entry.name_hash).map(|p| Kind::of_path(p));
        let mut f = match File::open(&arc.arc_path) { Ok(f) => f, Err(e) => { self.preview = Some(Preview::Error(e.to_string())); return; } };
        let data = match tab::decode_entry(&mut f, &arc.tab, &entry, &self.oodle_dll, &mut self.oodle) {
            Ok(d) => d, Err(e) => { self.preview = Some(Preview::Error(format!("decode: {e}"))); return; }
        };
        self.preview = Some(build_preview(&data, path_ext));
    }
}

fn build_preview(data: &[u8], path_ext: Option<Kind>) -> Preview {
    // resourcebundle has no magic — recognized only by path extension
    if path_ext == Some(Kind::Structure) {
        return Preview::Structure(build_bundle(data));
    }
    match tab::magic_ext(data) {
        "sarc" => match sarc::parse(data) {
            Ok(members) => {
                if members.iter().any(|m| m.stored && m.name.ends_with(".modelc")) {
                    Preview::Model { parts: build_model(data, &members) }
                } else {
                    let stored = members.iter().filter(|m| m.stored).count();
                    Preview::Sarc(SarcView { stored, refs: members.len() - stored,
                        members: members.iter().map(|m| (m.name.clone(), m.size, m.stored)).collect() })
                }
            }
            Err(e) => Preview::Error(e),
        },
        "avtx" => match avtx::parse_header(data) {
            Ok(h) => {
                let mip = avtx::best_inline_mip(data).ok();
                let dims = mip.map(|m| (m.width, m.height, avtx::dxgi_name(m.dxgi_format).to_string()));
                let info = format!("{}x{}x{}  {} ({:#x})\nmips {}/{} inline{}{}",
                    h.width, h.height, h.depth, avtx::dxgi_name(h.dxgi_format), h.dxgi_format,
                    h.mip_inline, h.mip_total, if h.is_cubemap() { "  [cubemap]" } else { "" },
                    if h.has_external_mips() { "  [external hi-res]" } else { "" });
                Preview::Texture(TexView { info, dims })
            }
            Err(e) => Preview::Error(e),
        },
        "adf" => match adf::parse(data.to_vec()) {
            Ok(a) => Preview::Data(serde_json::to_string_pretty(&a.decode_instances()).unwrap_or_default()),
            Err(e) => Preview::Error(e),
        },
        _ => Preview::Raw(hexdump(data, 768)),
    }
}

fn build_bundle(data: &[u8]) -> BundleView {
    let mut members = Vec::new();
    for m in bundle::parse(data) {
        let d = m.data(data);
        let ext = tab::magic_ext(d).to_string();
        let mut fields = Vec::new();
        if ext == "adf" {
            if let Ok(a) = adf::parse(d.to_vec()) {
                let v = a.decode_instances();
                if let Some(root) = v.as_object() {
                    for (name, val) in root {
                        fields.push((format!("‹{name}›"), String::new()));
                        if let Some(o) = val.as_object() {
                            for (k, fv) in o {
                                let s = if let Some(arr) = fv.as_array() { format!("[{}]", arr.len()) }
                                    else if let Some(o2) = fv.as_object() { format!("{{{}}}", o2.len()) }
                                    else { let t = fv.to_string(); if t.len() > 40 { t[..40].to_string() } else { t } };
                                fields.push((k.clone(), s));
                            }
                        }
                    }
                }
            }
        }
        members.push((m.name_hash, m.type_hash, m.size, ext, fields));
    }
    BundleView { members }
}

fn build_model(data: &[u8], members: &[sarc::SarcEntry]) -> Vec<ModelPart> {
    let by_name: std::collections::HashMap<&str, &sarc::SarcEntry> = members.iter().map(|m| (m.name.as_str(), m)).collect();
    let mut parts = Vec::new();
    for mc in members.iter().filter(|m| m.stored && m.name.ends_with(".modelc")) {
        let bytes = match mc.data(data) { Some(b) => b, None => continue };
        let a = match adf::parse(bytes.to_vec()) { Ok(a) => a, Err(_) => continue };
        let v = a.decode_instances();
        let model = match v.as_object().and_then(|o| o.values().next()) { Some(m) => m, None => continue };
        let lods = model.get("LodSlots").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        let lod_factor = model.get("LodFactor").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let mut materials = Vec::new();
        if let Some(mats) = model.get("Materials").and_then(|v| v.as_array()) {
            for mat in mats {
                materials.push(Material {
                    name: mat.get("Name").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                    render_block: mat.get("RenderBlockId").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                    textures: mat.get("Textures").and_then(|v| v.as_array()).map(|ts| ts.iter()
                        .filter_map(|v| v.as_str()).filter(|t| !t.is_empty() && !t.contains("dummies/"))
                        .map(|t| basename(t).to_string()).collect()).unwrap_or_default(),
                });
            }
        }
        let mesh = model.get("Mesh").and_then(|v| v.as_str())
            .and_then(|mp| by_name.get(mp).and_then(|m| m.data(data)))
            .and_then(mesh_stat);
        parts.push(ModelPart { name: basename(&mc.name).to_string(), lods, lod_factor, materials, mesh });
    }
    parts
}

fn mesh_stat(data: &[u8]) -> Option<MeshStat> {
    let a = adf::parse(data.to_vec()).ok()?;
    let v = a.decode_instances();
    let hdr = v.as_object()?.values().find(|iv| iv.get("LodGroups").is_some())?;
    let groups = hdr.get("LodGroups").and_then(|v| v.as_array())?;
    let (mut meshes, mut verts, mut indices) = (0u64, 0u64, 0u64);
    for g in groups {
        if let Some(ms) = g.get("Meshes").and_then(|v| v.as_array()) {
            for m in ms {
                meshes += 1;
                verts += m.get("VertexCount").and_then(|v| v.as_u64()).unwrap_or(0);
                indices += m.get("IndexCount").and_then(|v| v.as_u64()).unwrap_or(0);
            }
        }
    }
    Some(MeshStat { groups: groups.len(), meshes, verts, indices,
        hires: hdr.get("HighLodPath").and_then(|v| v.as_str()).map(|s| basename(s).to_string()).unwrap_or_default() })
}

fn hexdump(b: &[u8], max: usize) -> String {
    let mut s = String::new();
    for (i, row) in b[..max.min(b.len())].chunks(16).enumerate() {
        let hex: String = row.iter().map(|x| format!("{:02x} ", x)).collect();
        let asc: String = row.iter().map(|&x| if (0x20..0x7f).contains(&x) { x as char } else { '.' }).collect();
        s.push_str(&format!("{:08x}  {:<48}{}\n", i * 16, hex, asc));
    }
    s
}

// ── small UI helpers ─────────────────────────────────────────────────────────────────────────────
fn eyebrow(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(2.0, 11.0), egui::Sense::hover());
        ui.painter().rect_filled(r, 0.0, ACC);
        ui.add_space(3.0);
        ui.label(RichText::new(text.to_uppercase()).color(DIM).size(10.0).strong());
    });
}

fn card(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none().fill(G2).stroke(Stroke::new(1.0, LINE)).rounding(6.0).inner_margin(egui::Margin::same(11.0))
        .show(ui, |ui| {
            eyebrow(ui, title);
            ui.add_space(6.0);
            body(ui);
        });
    ui.add_space(9.0);
}

fn rail_btn(ui: &mut egui::Ui, glyph: &str, label: &str, on: bool, soon: bool) -> bool {
    let color = if on { ACC } else if soon { FAINT } else { DIM };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 54.0),
        if soon { egui::Sense::hover() } else { egui::Sense::click() });
    if on { ui.painter().rect_filled(egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height())), 0.0, ACC); }
    if resp.hovered() && !soon { ui.painter().rect_filled(rect, 0.0, Color32::from_white_alpha(4)); }
    let p = ui.painter();
    p.text(rect.center() - egui::vec2(0.0, 8.0), egui::Align2::CENTER_CENTER, glyph, egui::FontId::proportional(18.0), color);
    p.text(rect.center() + egui::vec2(0.0, 13.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(8.5), color);
    resp.clicked()
}

impl eframe::App for Workshop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // command bar
        egui::TopBottomPanel::top("cmd").exact_height(46.0)
            .frame(egui::Frame::none().fill(G0).inner_margin(egui::Margin::symmetric(14.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("JUST CAUSE 4").color(TX).size(14.0).strong());
                    ui.label(RichText::new("WORKSHOP").color(FAINT).size(9.0).strong());
                    ui.add_space(8.0);
                    ui.separator();
                    if let Some(a) = &self.archive {
                        ui.label(RichText::new(name_of(&a.tab_path)).color(DIM).monospace());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Filelist…").clicked() {
                            if let Some(p) = rfd::FileDialog::new().add_filter("filelist", &["filelist", "txt"]).pick_file() { self.load_filelist(p); }
                        }
                        if ui.button("Open .tab…").clicked() {
                            if let Some(p) = rfd::FileDialog::new().add_filter("TAB", &["tab"]).pick_file() { self.open_archive(p); }
                        }
                    });
                });
            });
        // status bar
        egui::TopBottomPanel::bottom("status").exact_height(26.0)
            .frame(egui::Frame::none().fill(G0).inner_margin(egui::Margin::symmetric(13.0, 5.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("●").color(VOLT).size(9.0));
                    ui.label(RichText::new(&self.status).color(DIM).size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(if self.oodle.is_some() { "Oodle: loaded" } else { "Oodle: idle" }).color(FAINT).monospace().size(11.0));
                        ui.label(RichText::new(format!("{} names", self.names.len())).color(FAINT).monospace().size(11.0));
                    });
                });
            });
        // rail
        egui::SidePanel::left("rail").exact_width(62.0).resizable(false)
            .frame(egui::Frame::none().fill(G0)).show(ctx, |ui| {
                ui.add_space(6.0);
                if rail_btn(ui, "🔍", "INSPECT", self.page == Page::Inspect, false) { self.page = Page::Inspect; }
                if rail_btn(ui, "⚙", "SETTINGS", self.page == Page::Settings, false) { self.page = Page::Settings; }
                ui.add_space(ui.available_height() - 108.0);
                rail_btn(ui, "🧊", "MODELS", false, true);
                rail_btn(ui, "📦", "PACK", false, true);
            });

        match self.page {
            Page::Inspect => self.ui_inspect(ctx),
            Page::Settings => self.ui_settings(ctx),
        }
    }
}

impl Workshop {
    fn ui_inspect(&mut self, ctx: &egui::Context) {
        // navigator (archive browser)
        egui::SidePanel::left("nav").default_width(310.0).frame(egui::Frame::none().fill(G1).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.label(RichText::new("ARCHIVE").color(TX).size(15.0).strong());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("filter").color(FAINT).size(10.0));
                    ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(f32::INFINITY).hint_text("name or hash"));
                });
                ui.add_space(6.0);
                let mut pick = None;
                match &self.archive {
                    None => { ui.add_space(12.0); ui.label(RichText::new("Open a .tab to browse.").color(FAINT)); }
                    Some(a) => {
                        let f = self.filter.to_lowercase();
                        let visible: Vec<usize> = a.tab.entries.iter().enumerate().filter(|(_, e)| {
                            if f.is_empty() { return true; }
                            self.names.get(&e.name_hash).map_or(false, |n| n.to_lowercase().contains(&f))
                                || format!("{:08x}", e.name_hash).contains(&f)
                        }).map(|(i, _)| i).collect();
                        eyebrow(ui, &format!("entries · {}", visible.len()));
                        ui.add_space(4.0);
                        let row_h = 20.0;
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_h, visible.len(), |ui, range| {
                            for row in range {
                                let i = visible[row];
                                let e = &a.tab.entries[i];
                                let (tag, tcol) = self.entry_kind(e).tag();
                                let sel = self.selected == Some(i);
                                let resp = ui.horizontal(|ui| {
                                    ui.label(RichText::new(tag).color(tcol).size(8.5).strong());
                                    let lbl = RichText::new(self.entry_label(e)).monospace().size(11.5)
                                        .color(if sel { ACC } else { TX });
                                    ui.add(egui::Label::new(lbl).truncate())
                                }).response;
                                if ui.interact(resp.rect, egui::Id::new(("row", i)), egui::Sense::click()).clicked() { pick = Some(i); }
                            }
                        });
                    }
                }
                if let Some(i) = pick { self.select(i); }
            });

        // inspector
        egui::SidePanel::right("insp").default_width(372.0).frame(egui::Frame::none().fill(G1).inner_margin(13.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.ui_inspector(ui));
            });

        // viewport (stub)
        egui::CentralPanel::default().frame(egui::Frame::none().fill(G0)).show(ctx, |ui| {
            let dims = match &self.preview {
                Some(Preview::Texture(t)) => t.dims.clone(),
                _ => None,
            };
            viewport(ui, dims.as_ref(), self.sel_kind);
        });
    }

    fn ui_inspector(&self, ui: &mut egui::Ui) {
        match &self.preview {
            None => { ui.add_space(16.0); ui.label(RichText::new("Pick an entry to inspect.").color(FAINT)); }
            Some(Preview::Model { parts }) => {
                ui.label(RichText::new("MODEL").color(ACC).size(9.5).strong());
                ui.label(RichText::new(format!("{} part(s)", parts.len())).color(TX).size(18.0).strong());
                ui.add_space(8.0);
                for p in parts {
                    card(ui, &p.name, |ui| {
                        ui.label(RichText::new(format!("{} LOD(s) · factor {:.2}", p.lods, p.lod_factor)).monospace().size(11.5).color(DIM));
                        if let Some(m) = &p.mesh {
                            ui.label(RichText::new(format!("{} LOD grp · {} mesh · {} verts · {} idx", m.groups, m.meshes, m.verts, m.indices)).monospace().size(11.5).color(TX));
                            if !m.hires.is_empty() { ui.label(RichText::new(format!("hi-res: {}", m.hires)).monospace().size(10.5).color(FAINT)); }
                        }
                        for mat in &p.materials {
                            ui.add_space(4.0);
                            ui.label(RichText::new(format!("▸ {}  [{}]", mat.name, mat.render_block)).size(12.0).color(VOLT));
                            for t in &mat.textures { ui.label(RichText::new(format!("    {}", t)).monospace().size(10.5).color(DIM)); }
                        }
                    });
                }
            }
            Some(Preview::Structure(bv)) => {
                ui.label(RichText::new("STRUCTURE").color(VOLT).size(9.5).strong());
                ui.label(RichText::new(format!("{} member(s)", bv.members.len())).color(TX).size(18.0).strong());
                ui.add_space(8.0);
                for (nh, th, size, ext, fields) in &bv.members {
                    card(ui, &format!("{ext} · {size} B"), |ui| {
                        ui.label(RichText::new(format!("name {nh:08x}  type {th:08x}")).monospace().size(10.0).color(FAINT));
                        ui.add_space(3.0);
                        for (k, val) in fields {
                            if val.is_empty() { ui.label(RichText::new(k).size(11.5).color(VOLT).strong()); }
                            else { ui.horizontal(|ui| {
                                ui.label(RichText::new(k).monospace().size(11.0).color(DIM));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(val).monospace().size(11.0).color(TX)));
                            }); }
                        }
                    });
                }
            }
            Some(Preview::Sarc(sv)) => {
                ui.label(RichText::new("BUNDLE (SARC)").color(INFO).size(9.5).strong());
                ui.label(RichText::new(format!("{} stored · {} ref", sv.stored, sv.refs)).color(TX).size(16.0).strong());
                ui.add_space(6.0);
                egui::Frame::none().fill(G2).stroke(Stroke::new(1.0, LINE)).rounding(6.0).inner_margin(10.0).show(ui, |ui| {
                    for (name, size, stored) in &sv.members {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(if *stored { "•" } else { "→" }).color(if *stored { VOLT } else { FAINT }));
                            ui.add(egui::Label::new(RichText::new(basename(name)).monospace().size(10.5).color(if *stored { TX } else { FAINT })).truncate());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(format!("{size}")).monospace().size(10.0).color(FAINT)));
                        });
                    }
                });
            }
            Some(Preview::Texture(t)) => {
                ui.label(RichText::new("TEXTURE (AVTX)").color(VOLT).size(9.5).strong());
                ui.add_space(6.0);
                card(ui, "format", |ui| { ui.label(RichText::new(&t.info).monospace().size(12.0).color(TX)); });
            }
            Some(Preview::Data(json)) => {
                ui.label(RichText::new("DATA (ADF → JSON)").color(DIM).size(9.5).strong());
                ui.add_space(6.0);
                ui.label(RichText::new(json).monospace().size(11.0).color(TX));
            }
            Some(Preview::Raw(hex)) => {
                ui.label(RichText::new("RAW").color(DIM).size(9.5).strong());
                ui.add_space(6.0);
                ui.label(RichText::new(hex).monospace().size(10.5).color(DIM));
            }
            Some(Preview::Error(e)) => { ui.colored_label(Color32::from_rgb(0xe0, 0x43, 0x2b), e); }
        }
    }

    fn ui_settings(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::none().fill(G1).inner_margin(24.0)).show(ctx, |ui| {
            ui.label(RichText::new("SETTINGS").color(TX).size(20.0).strong());
            ui.add_space(4.0);
            ui.label(RichText::new("Where the game & tools live, and how the bench looks.").color(DIM).size(12.5));
            ui.add_space(14.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                card(ui, "paths", |ui| {
                    let game = tab::GAME_DIR;
                    setting_row(ui, "Game install", game, true);
                    setting_row(ui, "Oodle DLL", &self.oodle_dll, self.oodle.is_some());
                    let fl = if self.names.is_empty() { "(load a filelist to un-hash names)".to_string() } else { format!("{} names loaded", self.names.len()) };
                    setting_row(ui, "Filelist", &fl, !self.names.is_empty());
                });
                card(ui, "rendering", |ui| {
                    setting_row(ui, "Backend", "wgpu", true);
                    setting_row(ui, "Viewport", "render pass — pending (stubbed)", false);
                });
                card(ui, "appearance", |ui| {
                    setting_row(ui, "Theme", "Storm dark", true);
                });
                card(ui, "about · v0.1.0", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for f in ["TAB/ARC v2", "Oodle", "ADF", "AVTX", "SARC", "resourcebundle", "name hash"] {
                            egui::Frame::none().fill(Color32::from_rgb(0x0e, 0x24, 0x29)).stroke(Stroke::new(1.0, Color32::from_rgb(0x2b, 0x8f, 0x96)))
                                .rounding(999.0).inner_margin(egui::Margin::symmetric(9.0, 3.0)).show(ui, |ui| {
                                    ui.label(RichText::new(f).monospace().size(10.5).color(VOLT));
                                });
                        }
                    });
                });
            });
        });
    }
}

fn setting_row(ui: &mut egui::Ui, label: &str, value: &str, ok: bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label.to_uppercase()).color(DIM).size(9.5).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ok { ui.label(RichText::new("✓").color(VOLT).size(11.0)); }
            ui.add(egui::Label::new(RichText::new(value).monospace().size(11.0).color(TX)).truncate());
        });
    });
    ui.add_space(3.0);
}

/// Render surface — STUB. The wgpu paint callback that uploads a texture mip / draws a model goes here.
fn viewport(ui: &mut egui::Ui, dims: Option<&(u32, u32, String)>, kind: Option<Kind>) {
    let rect = ui.available_rect_before_wrap();
    let p = ui.painter();
    p.rect_filled(rect, 0.0, G0);
    let inner = rect.shrink(14.0);
    p.rect(inner, 6.0, Color32::from_rgb(0x10, 0x18, 0x1e), Stroke::new(1.5, LINE));
    let msg = match (kind, dims) {
        (Some(Kind::Texture), Some((w, h, f))) => format!("◍  render viewport — not wired yet\n{w}×{h} · {f}\n(next: wgpu upload of the mip)"),
        (Some(Kind::Model), _) => "◍  model viewport — not wired yet\n(next: AMF geometry decode → mesh render)".to_string(),
        _ => "◍  render viewport — not wired yet".to_string(),
    };
    p.text(inner.center(), egui::Align2::CENTER_CENTER, msg, egui::FontId::monospace(13.0), FAINT);
    p.text(inner.left_top() + egui::vec2(11.0, 13.0), egui::Align2::LEFT_CENTER, "RENDER · PENDING", egui::FontId::proportional(10.0), INFO);
}
