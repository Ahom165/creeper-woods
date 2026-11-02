// Rastérisation de texte (ab_glyph → atlas) avec cache par (police, taille).
// Le texte écran en français : ASCII + accents latins étendus.

use crate::atlas::{Atlas, Uv};
use ab_glyph::{Font, FontArc, Glyph, PxScale, PxScaleFont, ScaleFont};
use std::collections::HashMap;

/// Jeu de caractères pré-rastérisé (ASCII + accents français + ponctuation).
pub const CHARSET: &str = concat!(
    " !\"#$%&'()*+,-./0123456789:;<=>?@",
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`",
    "abcdefghijklmnopqrstuvwxyz{|}~",
    "àâäéèêëîïôöùûüçÀÂÄÉÈÊËÎÏÔÖÙÛÜÇŒœ",
    "«»°…’‘–—·×'’"
);

pub const FONT_REGULAR: u8 = 0;
pub const FONT_BOLD: u8 = 1;

/// Tailles de texte pré-rastérisées (px logiques).
pub const SIZES: &[u32] = &[14, 16, 18, 22, 26, 32, 44, 60, 80];

#[derive(Clone, Copy, Debug)]
pub struct GlyphInfo {
    pub uv: Uv,
    pub w: f32,
    pub h: f32,
    /// Décalage du coin haut-gauche par rapport au curseur (y vers le bas).
    pub off_x: f32,
    pub off_y: f32,
    pub advance: f32,
}

pub struct TextEngine {
    pages: HashMap<(u8, u32), HashMap<char, GlyphInfo>>,
    line_height: HashMap<(u8, u32), f32>,
}

impl TextEngine {
    /// Charge les polices et rastérise tous les glyphes dans l'atlas.
    pub fn new(atlas: &mut Atlas) -> Self {
        let regular_bytes = std::fs::read("assets/fonts/DejaVuSans.ttf")
            .expect("police manquante : assets/fonts/DejaVuSans.ttf");
        let bold_bytes = std::fs::read("assets/fonts/DejaVuSans-Bold.ttf")
            .expect("police manquante : assets/fonts/DejaVuSans-Bold.ttf");
        let fonts = [
            FontArc::try_from_vec(regular_bytes).expect("police régulière invalide"),
            FontArc::try_from_vec(bold_bytes).expect("police grasse invalide"),
        ];
        let mut pages = HashMap::new();
        let mut line_height = HashMap::new();
        for (fi, font) in fonts.iter().enumerate() {
            for &size in SIZES {
                let scaled = PxScaleFont {
                    font: font.clone(),
                    scale: PxScale::from(size as f32),
                };
                let mut page = HashMap::new();
                for ch in CHARSET.chars() {
                    let id = font.glyph_id(ch);
                    if id.0 == 0 && ch != ' ' {
                        continue; // glyphe absent
                    }
                    let advance = scaled.h_advance(id);
                    if ch == ' ' {
                        page.insert(
                            ch,
                            GlyphInfo {
                                uv: Uv { u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0 },
                                w: 0.0,
                                h: 0.0,
                                off_x: 0.0,
                                off_y: 0.0,
                                advance,
                            },
                        );
                        continue;
                    }
                    let glyph = Glyph {
                        id,
                        scale: PxScale::from(size as f32),
                        position: ab_glyph::Point { x: 0.0, y: 0.0 },
                    };
                    let Some(outlined) = font.outline_glyph(glyph) else {
                        continue;
                    };
                    let bounds = outlined.px_bounds();
                    let w = bounds.width().ceil().max(1.0) as u32;
                    let h = bounds.height().ceil().max(1.0) as u32;
                    let uv = atlas.alloc(w, h);
                    let mut px = vec![0u8; (w * h * 4) as usize];
                    outlined.draw(|x, y, alpha| {
                        if x < w && y < h {
                            let off = ((y * w + x) * 4) as usize;
                            let a = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
                            px[off] = 255;
                            px[off + 1] = 255;
                            px[off + 2] = 255;
                            px[off + 3] = a;
                        }
                    });
                    atlas.write(uv, w, h, &px);
                    page.insert(
                        ch,
                        GlyphInfo {
                            uv,
                            w: w as f32,
                            h: h as f32,
                            off_x: bounds.min.x,
                            off_y: bounds.min.y,
                            advance,
                        },
                    );
                }
                let ascent = scaled.ascent();
                let descent = scaled.descent();
                line_height.insert((fi as u8, size), ascent - descent);
                pages.insert((fi as u8, size), page);
            }
        }
        TextEngine {
            pages,
            line_height,
        }
    }

    fn page(&self, font: u8, size: u32) -> &HashMap<char, GlyphInfo> {
        self.pages.get(&(font, size)).unwrap_or_else(|| {
            self.pages
                .get(&(font, SIZES[0]))
                .expect("page de glyphes absente")
        })
    }

    /// Hauteur de ligne (ascent − descent) en px pour cette police/taille.
    pub fn line_height(&self, font: u8, size: u32) -> f32 {
        *self.line_height.get(&(font, size)).unwrap_or(&(size as f32))
    }

    /// Largeur en px d'une ligne à cette police/taille.
    pub fn width(&self, text: &str, font: u8, size: u32) -> f32 {
        let page = self.page(font, size);
        text.chars()
            .map(|c| page.get(&c).map_or(size as f32 * 0.3, |g| g.advance))
            .sum()
    }

    /// Génère les quads de glyphes (positions relatives au point de départ,
    /// y vers le bas). La découpe de ligne doit être faite par l'appelant.
    pub fn layout(
        &self,
        text: &str,
        font: u8,
        size: u32,
        out: &mut Vec<PlacedGlyph>,
    ) {
        let page = self.page(font, size);
        let mut x = 0.0f32;
        for ch in text.chars() {
            let Some(g) = page.get(&ch) else {
                x += size as f32 * 0.3;
                continue;
            };
            if g.w > 0.0 {
                out.push(PlacedGlyph {
                    info: *g,
                    x: x + g.off_x,
                    y: g.off_y,
                });
            }
            x += g.advance;
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct PlacedGlyph {
    pub info: GlyphInfo,
    pub x: f32,
    pub y: f32,
}
