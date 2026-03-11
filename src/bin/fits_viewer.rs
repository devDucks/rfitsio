use eframe::egui;
use rfitsio::endian;
use rfitsio::hdu::HDU;
use rfitsio::parsing::parse;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("FITS Viewer")
            .with_inner_size([900.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "FITS Viewer",
        options,
        Box::new(|_cc| Ok(Box::new(FitsViewer::default()))),
    )
}

#[derive(Default)]
struct FitsViewer {
    texture: Option<egui::TextureHandle>,
    status: String,
    zoom: f32,
}

impl FitsViewer {
    fn load_fits(&mut self, path: &std::path::Path, ctx: &egui::Context) {
        let path_str = path.to_string_lossy();
        match parse(&path_str) {
            Err(e) => {
                self.status = format!("Error: {e}");
                self.texture = None;
            }
            Ok(fits) => {
                let mut loaded = false;
                for hdu in &fits.hdus {
                    if let Some((width, height, pixels)) = extract_image(hdu) {
                        let color_image = pixels_to_color_image(width, height, &pixels);
                        self.texture = Some(ctx.load_texture(
                            "fits_image",
                            color_image,
                            egui::TextureOptions::LINEAR,
                        ));
                        self.status = format!(
                            "{}  —  {}×{} px",
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy(),
                            width,
                            height,
                        );
                        self.zoom = 1.0;
                        loaded = true;
                        break;
                    }
                }
                if !loaded {
                    self.status = "No displayable 2D image HDU found.".into();
                    self.texture = None;
                }
            }
        }
    }
}

impl eframe::App for FitsViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Open…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("FITS", &["fits", "fit", "fts"])
                        .pick_file()
                    {
                        self.load_fits(&path, ctx);
                    }
                }
                if self.status.is_empty() {
                    ui.label("No file loaded.");
                } else {
                    ui.label(&self.status);
                }
            });
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(texture) = &self.texture {
                // Ctrl+scroll or plain scroll to zoom.
                let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
                if scroll_delta != 0.0 {
                    self.zoom =
                        (self.zoom * (1.0 + scroll_delta * 0.002)).clamp(0.02, 50.0);
                }

                let size = texture.size_vec2() * self.zoom;
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(
                            texture.id(),
                            size,
                        ))
                        .sense(egui::Sense::hover()),
                    );
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Click \"Open…\" to load a FITS file.");
                });
            }
        });
    }
}

/// Extract a 2-D image from an HDU, returning `(width, height, f32_pixels)`.
///
/// Only HDUs with `NAXIS >= 2`, non-zero `NAXIS1`/`NAXIS2`, and a recognised
/// `BITPIX` are considered.  Multi-plane data (NAXIS > 2) use the first plane.
fn extract_image(hdu: &HDU) -> Option<(usize, usize, Vec<f32>)> {
    let mut bitpix: i32 = 0;
    let mut naxis: usize = 0;
    let mut naxis1: usize = 0;
    let mut naxis2: usize = 0;

    for h in &hdu.headers {
        if h.key[8] != b'=' {
            continue;
        }
        let keyword = std::str::from_utf8(&h.key[..8])
            .unwrap_or("")
            .trim_end();
        let raw_val = std::str::from_utf8(&h.value).unwrap_or("");
        let val = raw_val.split('/').next().unwrap_or("").trim();

        match keyword {
            "BITPIX" => bitpix = val.parse().unwrap_or(0),
            "NAXIS" => naxis = val.parse().unwrap_or(0),
            "NAXIS1" => naxis1 = val.parse().unwrap_or(0),
            "NAXIS2" => naxis2 = val.parse().unwrap_or(0),
            _ => {}
        }
    }

    if naxis < 2 || naxis1 == 0 || naxis2 == 0 {
        return None;
    }

    let bpp = (bitpix.unsigned_abs() / 8) as usize;
    let pixel_count = naxis1 * naxis2;
    let byte_count = pixel_count * bpp;
    let raw = &hdu.data.data;

    if raw.len() < byte_count {
        return None;
    }

    let data = &raw[..byte_count];
    let pixels: Vec<f32> = match bitpix {
        8 => data.iter().map(|&b| b as f32).collect(),
        16 => endian::be_bytes_to_i16(data)
            .into_iter()
            .map(|v| v as f32)
            .collect(),
        32 => endian::be_bytes_to_i32(data)
            .into_iter()
            .map(|v| v as f32)
            .collect(),
        64 => endian::be_bytes_to_i64(data)
            .into_iter()
            .map(|v| v as f32)
            .collect(),
        -32 => endian::be_bytes_to_f32(data),
        -64 => endian::be_bytes_to_f64(data)
            .into_iter()
            .map(|v| v as f32)
            .collect(),
        _ => return None,
    };

    Some((naxis1, naxis2, pixels))
}

/// Linear-stretch normalise `pixels` to [0, 255] and pack into an egui image.
fn pixels_to_color_image(width: usize, height: usize, pixels: &[f32]) -> egui::ColorImage {
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &v in pixels {
        if v.is_finite() {
            if v < lo {
                lo = v;
            }
            if v > hi {
                hi = v;
            }
        }
    }
    let range = if (hi - lo).abs() < f32::EPSILON {
        1.0
    } else {
        hi - lo
    };

    let gray: Vec<egui::Color32> = pixels
        .iter()
        .map(|&v| {
            let n = ((v - lo) / range * 255.0).clamp(0.0, 255.0) as u8;
            egui::Color32::from_gray(n)
        })
        .collect();

    egui::ColorImage {
        size: [width, height],
        pixels: gray,
    }
}
