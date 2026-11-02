/// Rastérisation de police (ab_glyph) → atlas de glyphes + mise en page.
/// Une seule taille rastérisée (32 px), redimensionnée au dessin (échantillonnage
/// nearest → rendu « pixel-art » cohérent avec le style du jeu).
use ab_glyph::{Font as _, FontArc, Glyph};
use bytemuck::{Pod, Zeroable};

pub const ATLAS: usize = 1024;
pub const RASTER_PX: f32 = 32.0;

pub const CHARSET: &str = concat!(
    " !\"#$%&'()*+,-./0123456789:;<=>?@",
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`",
    "abcdefghijklmnopqrstuvwxyz{|}~",
    "«»×°€…’‘“”–—éèêëàâäîïôöùûüçœæ",
    "ÉÈÊËÀÂÄÎÏÔÖÙÛÜÇŒÆ",
);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct UiVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub struct GlyphInfo {
    /// Position du glyphe dans l'atlas, en pixels.
    pub uv: [f32; 2],
    /// Taille de l'encre, en pixels d'atlas.
    pub size: [f32; 2],
    /// Décalage depuis la plume (basé sur l'origine bas ligne), en pixels d'atlas.
    pub bearing: [f32; 2],
    pub advance: f32,
}

pub struct TextRenderer {
    pub image: image::RgbaImage,
    pub glyphs: std::collections::HashMap<char, GlyphInfo>,
}

impl TextRenderer {
    pub fn new(bytes: Vec<u8>) -> Self {
        let font = FontArc::try_from_vec(bytes).expect("police DejaVu incluse valide");
        let mut image = image::RgbaImage::new(ATLAS as u32, ATLAS as u32);
        let mut glyphs = std::collections::HashMap::new();
        let mut cx = 1usize;
        let mut cy = 1usize;
        let mut row_h = 0usize;
        // Échelle px-unités de la police (méthodes unscaled de ab_glyph).
        let s = RASTER_PX / font.units_per_em().unwrap_or(1000.0);

        for c in CHARSET.chars() {
            let g = Glyph {
                id: font.glyph_id(c),
                scale: RASTER_PX.into(),
                position: ab_glyph::point(0.0, 0.0),
            };
            let advance = font.h_advance_unscaled(g.id) * s;
            let Some(og) = font.outline_glyph(g) else {
                glyphs.insert(
                    c,
                    GlyphInfo {
                        uv: [0.0; 2],
                        size: [0.0; 2],
                        bearing: [0.0; 2],
                        advance,
                    },
                );
                continue;
            };
            let ink = og.px_bounds();
            let w = ink.width().ceil().max(1.0) as usize;
            let h = ink.height().ceil().max(1.0) as usize;
            if cx + w + 1 > ATLAS {
                cx = 1;
                cy += row_h + 1;
                row_h = 0;
            }
            // x/y du callback sont déjà relatifs au coin des bounds.
            og.draw(|x, y, alpha| {
                let px = cx as u32 + x;
                let py = cy as u32 + y;
                if px < ATLAS as u32 && py < ATLAS as u32 {
                    image.get_pixel_mut(px, py).0 = [255, 255, 255, (alpha * 255.0) as u8];
                }
            });
            glyphs.insert(
                c,
                GlyphInfo {
                    uv: [cx as f32, cy as f32],
                    size: [w as f32, h as f32],
                    bearing: [ink.min.x, ink.min.y],
                    advance,
                },
            );
            cx += w + 1;
            row_h = row_h.max(h);
        }

        Self {
            image,
            glyphs,
        }
    }

    pub fn measure(&self, text: &str, px: f32) -> f32 {
        let s = px / RASTER_PX;
        text.chars().filter_map(|c| self.glyphs.get(&c)).map(|g| g.advance * s).sum()
    }

    /// Empile les quads du texte dans `out`. `align` : 0 gauche, 1 centre, 2 droite.
    #[allow(clippy::too_many_arguments)] // alignement d'appels UI
    pub fn push_text(
        &self,
        out: &mut Vec<UiVertex>,
        text: &str,
        mut x: f32,
        y_baseline: f32,
        px: f32,
        color: [f32; 4],
        align: u32,
    ) {
        let s = px / RASTER_PX;
        if align > 0 {
            let w = self.measure(text, px);
            x -= if align == 1 { w * 0.5 } else { w };
        }
        for c in text.chars() {
            let Some(g) = self.glyphs.get(&c) else { continue };
            let gx = x + g.bearing[0] * s;
            let gy = y_baseline + g.bearing[1] * s;
            let gw = g.size[0] * s;
            let gh = g.size[1] * s;
            if g.size[0] > 0.0 {
                push_quad_raw(
                    out,
                    gx,
                    gy,
                    gw,
                    gh,
                    g.uv[0] / ATLAS as f32,
                    g.uv[1] / ATLAS as f32,
                    g.uv[0] / ATLAS as f32 + g.size[0] / ATLAS as f32,
                    g.uv[1] / ATLAS as f32 + g.size[1] / ATLAS as f32,
                    color,
                );
            }
            x += g.advance * s;
        }
    }
}

/// Ajoute un quad UI : positions en pixels de la résolution de conception 1280×720,
/// UV normalisées, couleur multipliée par la texture.
#[allow(clippy::too_many_arguments)]
pub fn push_quad_raw(
    out: &mut Vec<UiVertex>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    c: [f32; 4],
) {
    let v = |px: f32, py: f32, u: f32, vv: f32| UiVertex {
        pos: [px, py],
        uv: [u, vv],
        color: c,
    };
    out.push(v(x, y, u0, v0));
    out.push(v(x + w, y, u1, v0));
    out.push(v(x + w, y + h, u1, v1));
    out.push(v(x, y, u0, v0));
    out.push(v(x + w, y + h, u1, v1));
    out.push(v(x, y + h, u0, v1));
}
