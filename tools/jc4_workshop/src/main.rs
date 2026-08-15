// jc4_workshop — JC4 asset browser & inspector (wgpu + egui).
//
// SCAFFOLD: archive loading, entry browsing (un-hashed via a filelist), and asset inspection
// (AVTX header, ADF -> JSON, hex) all work. The model/texture RENDER viewport is intentionally a
// placeholder — the GPU render pass slots into `texture_viewport()` next. See ../../docs/tools/README.md.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use jc4_formats::{adf, avtx, hash::hashlittle, oodle::Oodle, tab};

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("JC4 Workshop"),
        ..Default::default()
    };
    eframe::run_native(
        "jc4_workshop",
        opts,
        Box::new(|_cc| {
            let mut app = Workshop::default();
            app.oodle_dll = tab::default_oodle_dll();
            Ok(Box::new(app))
        }),
    )
}

struct Archive {
    tab_path: PathBuf,
    arc_path: PathBuf,
    tab: tab::Tab,
}

enum Preview {
    Texture { info: String, dims: Option<(u32, u32, String)> },
    Adf { json: String },
    Raw { info: String, hex: String },
    Error(String),
}

#[derive(Default)]
struct Workshop {
    archive: Option<Archive>,
    names: HashMap<u32, String>, // name_hash -> resource path (from a filelist)
    filter: String,
    selected: Option<usize>,
    preview: Option<Preview>,
    status: String,
    oodle_dll: String,
    oodle: Option<Oodle>,
}

impl Workshop {
    fn open_archive(&mut self, tab_path: PathBuf) {
        let arc_path = tab_path.with_extension("arc");
        match std::fs::read(&tab_path).map_err(|e| e.to_string()).and_then(|b| tab::parse_tab(&b)) {
            Ok(t) => {
                self.status = format!("loaded {} entries from {}", t.entries.len(), name_of(&tab_path));
                self.archive = Some(Archive { tab_path, arc_path, tab: t });
                self.selected = None;
                self.preview = None;
            }
            Err(e) => self.status = format!("open failed: {e}"),
        }
    }

    fn load_filelist(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(txt) => {
                let mut n = 0;
                for line in txt.lines() {
                    let p = line.trim();
                    if p.is_empty() || p.starts_with(';') { continue; }
                    self.names.insert(hashlittle(p.as_bytes(), 0), p.to_string());
                    n += 1;
                }
                self.status = format!("filelist: {n} paths ({} known names total)", self.names.len());
            }
            Err(e) => self.status = format!("filelist failed: {e}"),
        }
    }

    fn select(&mut self, i: usize) {
        self.selected = Some(i);
        // disjoint field borrows: &self.archive (shared), &mut self.oodle (mut), &self.oodle_dll (shared)
        let arc = match &self.archive { Some(a) => a, None => return };
        let entry = arc.tab.entries[i];
        let mut f = match File::open(&arc.arc_path) {
            Ok(f) => f,
            Err(e) => { self.preview = Some(Preview::Error(format!("open arc: {e}"))); return; }
        };
        let decoded = tab::decode_entry(&mut f, &arc.tab, &entry, &self.oodle_dll, &mut self.oodle);
        self.preview = Some(match decoded {
            Ok(data) => build_preview(&data),
            Err(e) => Preview::Error(format!("decode: {e}")),
        });
    }

    fn entry_label(&self, e: &tab::Entry) -> String {
        match self.names.get(&e.name_hash) {
            Some(p) => p.clone(),
            None => format!("{:08x}.{}", e.name_hash, if e.codec() == 0 { "raw" } else { "z" }),
        }
    }
}

fn name_of(p: &Path) -> String { p.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string() }

fn build_preview(data: &[u8]) -> Preview {
    match tab::magic_ext(data) {
        "avtx" => match avtx::parse_header(data) {
            Ok(h) => {
                let mip = avtx::best_inline_mip(data).ok();
                let dims = mip.as_ref().map(|m| (m.width, m.height, avtx::dxgi_name(m.dxgi_format).to_string()));
                let info = format!(
                    "AVTX  {}x{}x{}  {} ({:#x})\nmips {}/{} inline{}{}\nsize_body {}  best inline mip {}",
                    h.width, h.height, h.depth, avtx::dxgi_name(h.dxgi_format), h.dxgi_format,
                    h.mip_inline, h.mip_total,
                    if h.is_cubemap() { "  [cubemap]" } else { "" },
                    if h.has_external_mips() { "  [external hi-res mips]" } else { "" },
                    h.size_body,
                    dims.as_ref().map(|(w, hh, f)| format!("{w}x{hh} {f}")).unwrap_or_else(|| "n/a".into()),
                );
                Preview::Texture { info, dims }
            }
            Err(e) => Preview::Error(e),
        },
        "adf" => match adf::parse(data.to_vec()) {
            Ok(a) => Preview::Adf { json: serde_json::to_string_pretty(&a.decode_instances()).unwrap_or_default() },
            Err(e) => Preview::Error(e),
        },
        ext => Preview::Raw { info: format!("{} bytes — type '{}'", data.len(), ext), hex: hexdump(data, 1024) },
    }
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

impl eframe::App for Workshop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- top menu bar ---
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Open archive (.tab)…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().add_filter("TAB", &["tab"]).pick_file() {
                        self.open_archive(p);
                    }
                }
                if ui.button("Load filelist…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().add_filter("filelist", &["filelist", "txt"]).pick_file() {
                        self.load_filelist(p);
                    }
                }
                ui.separator();
                if let Some(a) = &self.archive {
                    ui.label(format!("{}  ({} entries)", name_of(&a.tab_path), a.tab.entries.len()));
                }
            });
        });

        // --- bottom status bar ---
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(if self.oodle.is_some() { "Oodle: loaded" } else { "Oodle: idle" });
                    ui.separator();
                    ui.label(format!("{} names", self.names.len()));
                });
            });
        });

        // --- left: entry browser ---
        egui::SidePanel::left("browser").resizable(true).default_width(360.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("name or hash"));
            });
            ui.separator();
            let visible: Vec<usize> = match &self.archive {
                None => Vec::new(),
                Some(a) => {
                    let f = self.filter.to_lowercase();
                    a.tab.entries.iter().enumerate().filter(|(_, e)| {
                        if f.is_empty() { return true; }
                        self.names.get(&e.name_hash).map_or(false, |n| n.to_lowercase().contains(&f))
                            || format!("{:08x}", e.name_hash).contains(&f)
                    }).map(|(i, _)| i).collect()
                }
            };
            let row_h = ui.text_style_height(&egui::TextStyle::Body);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_h, visible.len(), |ui, range| {
                if let Some(a) = &self.archive {
                    for row in range {
                        let i = visible[row];
                        let label = self.entry_label(&a.tab.entries[i]);
                        if ui.selectable_label(self.selected == Some(i), label).clicked() {
                            // defer the actual decode until after the borrow of `a` ends
                            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("pick"), i));
                        }
                    }
                }
            });
        });

        // apply a deferred pick (decode outside the archive borrow above)
        if let Some(i) = ctx.data_mut(|d| d.remove_temp::<usize>(egui::Id::new("pick"))) {
            self.select(i);
        }

        // --- center: inspector ---
        egui::CentralPanel::default().show(ctx, |ui| {
            match &self.preview {
                None => { ui.centered_and_justified(|ui| { ui.label("Open an archive, then pick an entry."); }); }
                Some(Preview::Texture { info, dims }) => {
                    ui.heading("Texture (AVTX)");
                    ui.monospace(info);
                    ui.separator();
                    texture_viewport(ui, dims.as_ref());
                }
                Some(Preview::Adf { json }) => {
                    ui.heading("Data (ADF → JSON)");
                    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                        ui.monospace(json);
                    });
                }
                Some(Preview::Raw { info, hex }) => {
                    ui.heading("Raw");
                    ui.label(info);
                    ui.separator();
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| { ui.monospace(hex); });
                }
                Some(Preview::Error(e)) => { ui.colored_label(egui::Color32::LIGHT_RED, e); }
            }
        });
    }
}

/// The render surface. STUB: draws a placeholder frame + the target dims/format. The wgpu paint
/// callback that uploads the BCn mip (jc4_formats::avtx) and blits it goes here next.
fn texture_viewport(ui: &mut egui::Ui, dims: Option<&(u32, u32, String)>) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ui.available_height().max(200.0)), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 4.0, egui::Color32::from_gray(24));
    p.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)));
    let msg = match dims {
        Some((w, h, f)) => format!("◍  render viewport — not wired yet\n{w}×{h}  {f}\n(next: wgpu upload of the BCn mip)"),
        None => "◍  render viewport — no decodable mip".to_string(),
    };
    p.text(rect.center(), egui::Align2::CENTER_CENTER, msg, egui::FontId::monospace(14.0), egui::Color32::from_gray(150));
}
