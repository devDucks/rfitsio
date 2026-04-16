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

struct FitsViewer {
    texture: Option<egui::TextureHandle>,
    /// Raw f32 pixel data: flat layout — plane0 | plane1 | plane2 (if nplanes == 3).
    raw_pixels: Option<Vec<f32>>,
    /// 1 for monochrome, 3 when the HDU has NAXIS3 == 3.
    raw_nplanes: usize,
    raw_width: usize,
    raw_height: usize,
    /// Sorted finite values per channel for fast percentile lookups.
    /// len == raw_nplanes (1 or 3).
    sorted_channels: Vec<Vec<f32>>,
    /// Key metadata rows extracted from the primary HDU headers.
    fits_info: Vec<(String, String)>,
    show_info: bool,
    status: String,
    zoom: f32,
    /// Shadow clipping as a percentile of the pixel distribution [0, 50].
    black_pct: f32,
    /// Highlight clipping as a percentile [50, 100].
    white_pct: f32,
    /// Gamma correction exponent (1.0 = linear, <1 brightens midtones, >1 darkens).
    gamma: f32,
    /// When true and raw_nplanes == 3, render planes 0/1/2 as R/G/B.
    rgb_mode: bool,
}

impl Default for FitsViewer {
    fn default() -> Self {
        Self {
            texture: None,
            raw_pixels: None,
            raw_nplanes: 1,
            raw_width: 0,
            raw_height: 0,
            sorted_channels: Vec::new(),
            fits_info: Vec::new(),
            show_info: true,
            status: String::new(),
            zoom: 1.0,
            black_pct: 0.5,
            white_pct: 99.5,
            gamma: 1.0,
            rgb_mode: false,
        }
    }
}

/// Look up the pixel value at the given percentile (0–100) from a pre-sorted slice.
fn percentile_value(sorted: &[f32], pct: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((pct / 100.0) * (sorted.len().saturating_sub(1)) as f32).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Normalise a pixel value `v` to [0, 255] using the given lo/hi stretch and gamma.
fn stretch_pixel(v: f32, lo: f32, hi: f32, inv_gamma: f32) -> u8 {
    let stretch = if (hi - lo).abs() < f32::EPSILON {
        1.0
    } else {
        hi - lo
    };
    let normalised = ((v - lo) / stretch).clamp(0.0, 1.0);
    (normalised.powf(inv_gamma) * 255.0) as u8
}

impl FitsViewer {
    fn load_fits(&mut self, path: &std::path::Path, ctx: &egui::Context) {
        let path_str = path.to_string_lossy();
        match parse(&path_str) {
            Err(e) => {
                self.status = format!("Error: {e}");
                self.texture = None;
                self.raw_pixels = None;
                self.sorted_channels.clear();
                self.fits_info.clear();
            }
            Ok(fits) => {
                let mut loaded = false;
                for hdu in &fits.hdus {
                    if let Some((raw_w, raw_h, nplanes_hdu, pixels_hdu)) = extract_image(hdu) {
                        // Debayer single-plane Bayer images into 3 planes.
                        let (width, height, nplanes, pixels) = if nplanes_hdu == 1 {
                            if let Some(pat) = extract_bayer_pattern(hdu) {
                                let (dw, dh, dp) = debayer(&pixels_hdu, raw_w, raw_h, &pat);
                                (dw, dh, 3, dp)
                            } else {
                                (raw_w, raw_h, 1, pixels_hdu)
                            }
                        } else {
                            (raw_w, raw_h, nplanes_hdu, pixels_hdu)
                        };

                        // Build per-channel sorted lists for percentile lookups.
                        let plane_size = width * height;
                        let mut sorted_channels: Vec<Vec<f32>> = Vec::new();
                        for p in 0..nplanes {
                            let plane = &pixels[p * plane_size..(p + 1) * plane_size];
                            let mut s: Vec<f32> =
                                plane.iter().copied().filter(|v| v.is_finite()).collect();
                            s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                            sorted_channels.push(s);
                        }

                        self.sorted_channels = sorted_channels;
                        self.raw_nplanes = nplanes;
                        self.raw_width = width;
                        self.raw_height = height;
                        self.raw_pixels = Some(pixels);
                        self.fits_info = extract_fits_info(hdu);
                        self.show_info = true;
                        // Enable RGB automatically when 3 planes are available.
                        self.rgb_mode = nplanes == 3;

                        // Default: clip the extreme 0.5% on each end for a clean auto-stretch.
                        self.black_pct = 0.5;
                        self.white_pct = 99.5;
                        self.gamma = 1.0;
                        self.zoom = 1.0;

                        self.status = format!(
                            "{}  —  {}×{} px{}",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            width,
                            height,
                            if nplanes == 3 { "  (RGB)" } else { "" },
                        );

                        self.rebuild_texture(ctx);
                        loaded = true;
                        break;
                    }
                }
                if !loaded {
                    self.status = "No displayable 2D image HDU found.".into();
                    self.texture = None;
                    self.raw_pixels = None;
                    self.sorted_channels.clear();
                    self.fits_info.clear();
                }
            }
        }
    }

    /// Re-render the texture from raw pixels using the current slider values.
    fn rebuild_texture(&mut self, ctx: &egui::Context) {
        let pixels = match &self.raw_pixels {
            Some(p) => p,
            None => return,
        };

        let inv_gamma = 1.0 / self.gamma.max(0.01);
        let plane_size = self.raw_width * self.raw_height;

        let image_pixels: Vec<egui::Color32> = if self.rgb_mode
            && self.raw_nplanes >= 3
            && self.sorted_channels.len() >= 3
        {
            // Per-channel stretch: each plane is normalised independently so that
            // colour-balanced captures don't end up with one channel dominating.
            let (lo_r, hi_r) =
                channel_stretch_limits(&self.sorted_channels[0], self.black_pct, self.white_pct);
            let (lo_g, hi_g) =
                channel_stretch_limits(&self.sorted_channels[1], self.black_pct, self.white_pct);
            let (lo_b, hi_b) =
                channel_stretch_limits(&self.sorted_channels[2], self.black_pct, self.white_pct);

            (0..plane_size)
                .map(|i| {
                    let r = stretch_pixel(pixels[i], lo_r, hi_r, inv_gamma);
                    let g = stretch_pixel(pixels[plane_size + i], lo_g, hi_g, inv_gamma);
                    let b = stretch_pixel(pixels[2 * plane_size + i], lo_b, hi_b, inv_gamma);
                    egui::Color32::from_rgb(r, g, b)
                })
                .collect()
        } else {
            // Grayscale — use only the first (or only) plane.
            let lo = percentile_value(&self.sorted_channels[0], self.black_pct);
            let hi = percentile_value(&self.sorted_channels[0], self.white_pct);
            pixels[..plane_size]
                .iter()
                .map(|&v| {
                    let byte = stretch_pixel(v, lo, hi, inv_gamma);
                    egui::Color32::from_gray(byte)
                })
                .collect()
        };

        let color_image = egui::ColorImage {
            size: [self.raw_width, self.raw_height],
            source_size: egui::Vec2::new(self.raw_width as f32, self.raw_height as f32),
            pixels: image_pixels,
        };

        self.texture =
            Some(ctx.load_texture("fits_image", color_image, egui::TextureOptions::LINEAR));
    }
}

/// Return the (lo, hi) pixel values for a channel given the current percentile settings.
fn channel_stretch_limits(sorted: &[f32], black_pct: f32, white_pct: f32) -> (f32, f32) {
    (
        percentile_value(sorted, black_pct),
        percentile_value(sorted, white_pct),
    )
}

impl eframe::App for FitsViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);

            // Row 1 — file controls.
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

            // Row 2 — image adjustment sliders (visible only after loading).
            if self.raw_pixels.is_some() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let mut changed = false;

                    ui.label("Black %:");
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut self.black_pct, 0.0..=50.0)
                                .fixed_decimals(1)
                                .suffix("%"),
                        )
                        .changed();

                    ui.separator();

                    ui.label("White %:");
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut self.white_pct, 50.0..=100.0)
                                .fixed_decimals(1)
                                .suffix("%"),
                        )
                        .changed();

                    // Ensure black stays below white.
                    if self.black_pct >= self.white_pct {
                        self.white_pct = (self.black_pct + 0.1).min(100.0);
                        changed = true;
                    }

                    ui.separator();

                    ui.label("Gamma:");
                    changed |= ui
                        .add(egui::Slider::new(&mut self.gamma, 0.1..=5.0).fixed_decimals(2))
                        .changed();

                    ui.separator();

                    // RGB checkbox — only interactive when 3 planes are available.
                    let has_rgb = self.raw_nplanes >= 3;
                    let rgb_resp =
                        ui.add_enabled(has_rgb, egui::Checkbox::new(&mut self.rgb_mode, "RGB"));
                    let rgb_changed = rgb_resp.changed();
                    if !has_rgb {
                        rgb_resp.on_disabled_hover_text(
                            "RGB mode requires a 3-plane image (NAXIS3 = 3)",
                        );
                    }
                    changed |= rgb_changed;

                    ui.separator();

                    if ui.button("Reset").clicked() {
                        self.black_pct = 0.5;
                        self.white_pct = 99.5;
                        self.gamma = 1.0;
                        changed = true;
                    }

                    if changed {
                        self.rebuild_texture(ctx);
                    }
                });
            }

            ui.add_space(2.0);
        });

        // FITS metadata panel — bottom-left corner of the central area.
        if !self.fits_info.is_empty() && self.show_info {
            let mut open = self.show_info;
            egui::Window::new("FITS Info")
                .anchor(egui::Align2::LEFT_BOTTOM, [8.0, -8.0])
                .resizable(false)
                .collapsible(true)
                .default_open(true)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("fits_info_grid")
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for (label, value) in &self.fits_info {
                                ui.label(
                                    egui::RichText::new(label)
                                        .color(egui::Color32::from_rgb(160, 160, 200)),
                                );
                                ui.label(value);
                                ui.end_row();
                            }
                        });
                });
            self.show_info = open;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(texture) = &self.texture {
                // Scroll to zoom.
                let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
                if scroll_delta != 0.0 {
                    self.zoom = (self.zoom * (1.0 + scroll_delta * 0.002)).clamp(0.02, 50.0);
                }

                let size = texture.size_vec2() * self.zoom;
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(texture.id(), size))
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

/// Extract a 2-D (or 3-plane) image from an HDU.
///
/// Returns `(width, height, nplanes, flat_pixels)` where `flat_pixels` is laid out as
/// plane0 | plane1 | plane2.  `nplanes` is 1 for a plain 2-D image or 3 when
/// NAXIS3 == 3 (the standard layout for RGB FITS data).  Other NAXIS3 values are
/// treated as single-plane (only the first plane is returned).
///
/// BZERO and BSCALE are applied so returned values represent physical pixel values.
fn extract_image(hdu: &HDU) -> Option<(usize, usize, usize, Vec<f32>)> {
    let mut bitpix: i32 = 0;
    let mut naxis: usize = 0;
    let mut naxis1: usize = 0;
    let mut naxis2: usize = 0;
    let mut naxis3: usize = 0;
    let mut bzero: f64 = 0.0;
    let mut bscale: f64 = 1.0;

    for h in &hdu.headers {
        if h.key[8] != b'=' {
            continue;
        }
        let keyword = std::str::from_utf8(&h.key[..8]).unwrap_or("").trim_end();
        let raw_val = std::str::from_utf8(&h.value).unwrap_or("");
        let val = raw_val.split('/').next().unwrap_or("").trim();

        match keyword {
            "BITPIX" => bitpix = val.parse().unwrap_or(0),
            "NAXIS" => naxis = val.parse().unwrap_or(0),
            "NAXIS1" => naxis1 = val.parse().unwrap_or(0),
            "NAXIS2" => naxis2 = val.parse().unwrap_or(0),
            "NAXIS3" => naxis3 = val.parse().unwrap_or(0),
            "BZERO" => bzero = val.parse().unwrap_or(0.0),
            "BSCALE" => bscale = val.parse().unwrap_or(1.0),
            _ => {}
        }
    }

    if naxis < 2 || naxis1 == 0 || naxis2 == 0 {
        return None;
    }

    // Decide how many planes to decode: 3 only when the cube is exactly (W, H, 3).
    let nplanes = if naxis >= 3 && naxis3 == 3 { 3 } else { 1 };

    let bpp = (bitpix.unsigned_abs() / 8) as usize;
    let plane_size = naxis1 * naxis2;
    let byte_count = plane_size * nplanes * bpp;
    let raw = &hdu.data.data;

    if raw.len() < byte_count {
        return None;
    }

    let data = &raw[..byte_count];

    let mut pixels: Vec<f32> = match bitpix {
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

    // Apply BSCALE / BZERO (physical = BZERO + BSCALE * stored).
    // Skip the identity case to avoid a pointless pass over the data.
    if bscale != 1.0 || bzero != 0.0 {
        let bscale32 = bscale as f32;
        let bzero32 = bzero as f32;
        for v in &mut pixels {
            *v = bzero32 + bscale32 * *v;
        }
    }

    Some((naxis1, naxis2, nplanes, pixels))
}

/// Return the Bayer pattern string (e.g. "RGGB") if the HDU contains a BAYERPAT keyword.
fn extract_bayer_pattern(hdu: &HDU) -> Option<String> {
    for h in &hdu.headers {
        if h.key[8] != b'=' {
            continue;
        }
        let keyword = std::str::from_utf8(&h.key[..8]).unwrap_or("").trim_end();
        if keyword == "BAYERPAT" {
            let raw_val = std::str::from_utf8(&h.value).unwrap_or("");
            let val = raw_val
                .split('/')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('\'')
                .trim()
                .to_uppercase();
            if !val.is_empty() {
                return Some(val);
            }
        }
    }
    None
}

/// Debayer a single-plane Bayer image into three separate R, G, B planes via 2×2 binning.
///
/// Each 2×2 Bayer cell contributes one output pixel:
/// - R  = the red   cell sample
/// - G  = average of the two green cell samples
/// - B  = the blue  cell sample
///
/// Output is (out_width, out_height, flat_pixels) with layout R_plane | G_plane | B_plane,
/// each plane being out_width × out_height values.
fn debayer(pixels: &[f32], width: usize, height: usize, pattern: &str) -> (usize, usize, Vec<f32>) {
    // Map the 4 positions in the 2×2 cell (in reading order: TL, TR, BL, BR)
    // to channel indices: 0=R, 1=G, 2=B.
    let cell_channels: [usize; 4] = {
        let mut ch = [1usize; 4]; // default all green
        for (i, c) in pattern.chars().take(4).enumerate() {
            ch[i] = match c {
                'R' => 0,
                'B' => 2,
                _ => 1, // G or anything else
            };
        }
        ch
    };

    let out_w = width / 2;
    let out_h = height / 2;
    let plane = out_w * out_h;

    let mut r = vec![0.0f32; plane];
    let mut g = vec![0.0f32; plane];
    let mut b = vec![0.0f32; plane];
    // Count how many samples contribute to green (always 2, but keep it data-driven).
    let mut g_count = vec![0u8; plane];

    // Cell positions relative to the top-left of the 2×2 cell: (row_offset, col_offset)
    const CELL_POS: [(usize, usize); 4] = [(0, 0), (0, 1), (1, 0), (1, 1)];

    for oy in 0..out_h {
        for ox in 0..out_w {
            let out_idx = oy * out_w + ox;
            for (cell_i, &(dr, dc)) in CELL_POS.iter().enumerate() {
                let src = (oy * 2 + dr) * width + (ox * 2 + dc);
                let v = pixels[src];
                match cell_channels[cell_i] {
                    0 => r[out_idx] = v,
                    2 => b[out_idx] = v,
                    _ => {
                        g[out_idx] += v;
                        g_count[out_idx] += 1;
                    }
                }
            }
            // Average the two green samples.
            let gc = g_count[out_idx];
            if gc > 1 {
                g[out_idx] /= gc as f32;
            }
        }
    }

    let mut result = Vec::with_capacity(plane * 3);
    result.extend_from_slice(&r);
    result.extend_from_slice(&g);
    result.extend_from_slice(&b);
    (out_w, out_h, result)
}

/// Extract human-readable metadata rows from an HDU's headers.
fn extract_fits_info(hdu: &HDU) -> Vec<(String, String)> {
    const WANTED: &[(&str, &str)] = &[
        ("OBJECT", "Object"),
        ("TARGNAME", "Target"),
        ("DATE-OBS", "Date (obs)"),
        ("DATE-BEG", "Date (begin)"),
        ("EXPTIME", "Exposure"),
        ("EXPOSURE", "Exposure"),
        ("TELESCOP", "Telescope"),
        ("INSTRUME", "Instrument"),
        ("FILTER", "Filter"),
        ("FILTER1", "Filter 1"),
        ("FILTER2", "Filter 2"),
        ("OBSERVER", "Observer"),
        ("ORIGIN", "Origin"),
        ("RA", "RA"),
        ("DEC", "Dec"),
        ("EQUINOX", "Equinox"),
        ("BITPIX", "Bit depth"),
        ("NAXIS1", "Width (px)"),
        ("NAXIS2", "Height (px)"),
        ("BSCALE", "BScale"),
        ("BZERO", "BZero"),
    ];

    let mut kv: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for h in &hdu.headers {
        if h.key[8] != b'=' {
            continue;
        }
        let keyword = std::str::from_utf8(&h.key[..8])
            .unwrap_or("")
            .trim_end()
            .to_string();
        let raw_val = std::str::from_utf8(&h.value).unwrap_or("");
        let val = raw_val
            .split('/')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('\'')
            .trim()
            .to_string();
        if !val.is_empty() {
            kv.insert(keyword, val);
        }
    }

    let mut seen_labels: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut rows: Vec<(String, String)> = Vec::new();
    for &(key, label) in WANTED {
        if seen_labels.contains(label) {
            continue;
        }
        if let Some(val) = kv.get(key) {
            rows.push((label.to_string(), val.clone()));
            seen_labels.insert(label);
        }
    }
    rows
}
