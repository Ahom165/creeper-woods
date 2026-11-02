// Atlas de textures 2048×2048 : tuiles de terrain procédurales (ou vraies
// textures chargées depuis assets/textures/), icônes d'interface, sprites de
// particules. Tout le rendu utilise cet atlas unique (1 bind group).

use std::collections::HashMap;

pub const ATLAS_SIZE: u32 = 2048;
/// Taille standard d'une tuile de terrain dans l'atlas.
pub const TILE: u32 = 16;

/// Clés de tuiles de terrain (16×16). Le nom correspond au fichier attendu
/// dans assets/textures/ pour remplacer le fallback procédural.
pub const TERRAIN_KEYS: &[(&str, &str)] = &[
    ("grass_top", "herbe (dessus)"),
    ("grass_side", "herbe (côté)"),
    ("dirt", "terre"),
    ("stone", "pierre"),
    ("cobble", "pavé"),
    ("moss_stone", "pierre moussue (ruines)"),
    ("oak_log", "chêne (écorce)"),
    ("oak_log_top", "chêne (coupe)"),
    ("oak_leaves", "feuilles"),
    ("water", "eau"),
    ("sand", "sable"),
    ("path", "chemin en terre"),
    ("planks", "planches"),
    ("roof", "bardeaux"),
    ("dark_stone", "pierre sombre"),
    ("gate_wood", "bois de porte/barrière"),
    ("bush", "buisson"),
    ("flower_white", "fleur blanche"),
    ("flower_red", "fleur rouge"),
    ("mushroom", "champignon"),
    ("gravel", "gravier"),
    ("bones", "os (décor squelette)"),
    ("creeper_face", "face de creeper"),
    ("skull_face", "face de squelette"),
    ("zombie_face", "face de zombie"),
    ("villager_face", "face de villageois"),
    ("portal", "portail de sortie"),
    ("rune_stone", "pierre de rune"),
];

/// Clés d'icônes 24×24 (HUD et carte).
pub const ICON_KEYS: &[&str] = &[
    "heart",
    "wisp",
    "arrow",
    "emerald",
    "potion",
    "totem",
    "beacon",
    "firework",
    "sword",
    "bow",
    "armor",
    "map_player",
    "map_portal",
    "map_boss",
    "map_chest",
    "map_rune",
    "map_villager",
    "soft_dot",
    "spark",
    "smoke",
    "soul_wisp",
    "ring",
    "flame",
    "bubble",
    "flash",
    "aura",
    "white",
];

/// Rectangle UV dans l'atlas (normalisé 0..1).
#[derive(Clone, Copy, Debug)]
pub struct Uv {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

pub struct Atlas {
    pub pixels: Vec<u8>,
    rects: HashMap<&'static str, Uv>,
    cursor_x: u32,
    cursor_y: u32,
    row_h: u32,
}

impl Atlas {
    pub fn build() -> Self {
        let mut atlas = Atlas {
            pixels: vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize],
            rects: HashMap::new(),
            cursor_x: 0,
            cursor_y: 0,
            row_h: 0,
        };
        // 1. tuiles de terrain 16×16 (rangées de 16)
        for (i, (key, _desc)) in TERRAIN_KEYS.iter().enumerate() {
            let col = (i as u32) % 16;
            let row = (i as u32) / 16;
            let (x, y) = (col * TILE, row * TILE);
            let mut tile = [0u8; (TILE * TILE * 4) as usize];
            procedural_tile(key, &mut tile);
            atlas.blit(x, y, TILE, TILE, &tile);
            let uv = atlas.norm(x, y, TILE, TILE);
            atlas.rects.insert(*key, uv);
        }
        // 2. icônes 24×24 (rangées de 64)
        let icon_y0 = ((TERRAIN_KEYS.len() as u32).div_ceil(16)) * TILE;
        for (i, key) in ICON_KEYS.iter().enumerate() {
            let col = (i as u32) % 64;
            let row = (i as u32) / 64;
            let size = 24u32;
            let (x, y) = (col * size, icon_y0 + row * size);
            let mut icon = vec![0u8; (size * size * 4) as usize];
            procedural_icon(key, &mut icon, size);
            atlas.blit(x, y, size, size, &icon);
            let uv = atlas.norm(x, y, size, size);
            atlas.rects.insert(*key, uv);
        }
        // 3. zone de glyphes : allouée dynamiquement par text.rs au démarrage
        let icons_bottom = icon_y0 + (((ICON_KEYS.len() as u32).div_ceil(64)) * 24);
        atlas.cursor_y = icons_bottom;
        atlas.cursor_x = 0;
        atlas.row_h = 0;
        atlas
    }

    fn norm(&self, x: u32, y: u32, w: u32, h: u32) -> Uv {
        Uv {
            u0: x as f32 / ATLAS_SIZE as f32,
            v0: y as f32 / ATLAS_SIZE as f32,
            u1: (x + w) as f32 / ATLAS_SIZE as f32,
            v1: (y + h) as f32 / ATLAS_SIZE as f32,
        }
    }

    fn blit(&mut self, x: u32, y: u32, w: u32, h: u32, src: &[u8]) {
        for row in 0..h {
            let dst_off = (((y + row) * ATLAS_SIZE + x) * 4) as usize;
            let src_off = (row * w * 4) as usize;
            self.pixels[dst_off..dst_off + (w * 4) as usize]
                .copy_from_slice(&src[src_off..src_off + (w * 4) as usize]);
        }
    }

    /// Alloue une zone rectangulaire (emballage par étagères) — utilisé pour
    /// les glyphes de police. Panique si l'atlas est plein (ne devrait jamais
    /// arriver avec le jeu de glyphes fixe).
    pub fn alloc(&mut self, w: u32, h: u32) -> Uv {
        if self.cursor_x + w > ATLAS_SIZE {
            self.cursor_x = 0;
            self.cursor_y += self.row_h;
            self.row_h = 0;
        }
        assert!(
            self.cursor_y + h <= ATLAS_SIZE,
            "atlas de glyphes plein — augmentez ATLAS_SIZE"
        );
        let (x, y) = (self.cursor_x, self.cursor_y);
        self.cursor_x += w;
        self.row_h = self.row_h.max(h);
        self.norm(x, y, w, h)
    }

    /// Dessine des pixels dans une zone allouée (RGBA, alpha pré-multiplié non requis).
    pub fn write(&mut self, uv: Uv, w: u32, h: u32, src: &[u8]) {
        let x = (uv.u0 * ATLAS_SIZE as f32) as u32;
        let y = (uv.v0 * ATLAS_SIZE as f32) as u32;
        self.blit(x, y, w, h, src);
    }

    pub fn uv(&self, key: &str) -> Uv {
        *self
            .rects
            .get(key)
            .unwrap_or_else(|| panic!("clé d'atlas inconnue : {key}"))
    }

    /// Remplace une tuile par les pixels d'une texture utilisateur (16×16).
    pub fn replace_tile(&mut self, key: &str, pixels: &[u8]) {
        let uv = self.uv(key);
        let x = (uv.u0 * ATLAS_SIZE as f32) as u32;
        let y = (uv.v0 * ATLAS_SIZE as f32) as u32;
        self.blit(x, y, TILE, TILE, pixels);
    }
}

// ---------------------------------------------------------------------------
// Génération procédurale des tuiles (fallback pixel-art) et des icônes
// ---------------------------------------------------------------------------

/// PRNG déterministe simple (xorshift) pour un bruit stable.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 32) as u32
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo + 1) as u32) as i32
    }
}

fn put(px: &mut [u8], w: u32, x: u32, y: u32, c: [u8; 4]) {
    if x >= w || y >= w {
        return; // hors tuile (les peintres peuvent déborder d'un pixel)
    }
    let off = ((y * w + x) * 4) as usize;
    px[off..off + 4].copy_from_slice(&c);
}

fn fill(px: &mut [u8], w: u32, c: [u8; 4]) {
    for i in 0..(w * w) as usize {
        px[i * 4..i * 4 + 4].copy_from_slice(&c);
    }
}

fn noise_px(px: &mut [u8], w: u32, base: [u8; 4], amp: i32, seed: u64) {
    let mut rng = Rng(seed);
    for y in 0..w {
        for x in 0..w {
            let d = rng.range(-amp, amp);
            let mut c = base;
            for cc in c.iter_mut().take(3) {
                *cc = (*cc as i32 + d).clamp(0, 255) as u8;
            }
            put(px, w, x, y, c);
        }
    }
}

fn speckle(px: &mut [u8], w: u32, color: [u8; 4], count: u32, seed: u64) {
    let mut rng = Rng(seed);
    for _ in 0..count {
        let x = rng.range(0, w as i32 - 1) as u32;
        let y = rng.range(0, w as i32 - 1) as u32;
        put(px, w, x, y, color);
    }
}

/// Génère une tuile 16×16 procédurale pour la clé donnée.
pub fn procedural_tile(key: &str, out: &mut [u8]) {
    let w = TILE;
    match key {
        "grass_top" => {
            noise_px(out, w, [0x39, 0x6b, 0x2f, 0xff], 14, 1);
            speckle(out, w, [0x4f, 0x8a, 0x3d, 0xff], 42, 2);
            speckle(out, w, [0x2c, 0x54, 0x26, 0xff], 30, 3);
        }
        "grass_side" => {
            noise_px(out, w, [0x6b, 0x4f, 0x36, 0xff], 12, 4);
            for y in 0..4u32 {
                for x in 0..w {
                    let d = if y == 3 { 12 } else { 0 };
                    put(
                        out,
                        w,
                        x,
                        y,
                        [0x39 + d as u8, 0x6b + d as u8, 0x2f + d as u8, 0xff],
                    );
                }
            }
            speckle(out, w, [0x50, 0x3a, 0x28, 0xff], 24, 5);
        }
        "dirt" => {
            noise_px(out, w, [0x6b, 0x4f, 0x36, 0xff], 14, 6);
            speckle(out, w, [0x50, 0x3a, 0x28, 0xff], 36, 7);
        }
        "stone" => {
            noise_px(out, w, [0x8a, 0x8d, 0x90, 0xff], 10, 8);
            speckle(out, w, [0x74, 0x77, 0x7a, 0xff], 30, 9);
        }
        "cobble" => {
            noise_px(out, w, [0x83, 0x86, 0x89, 0xff], 8, 10);
            for (x, y) in [(2, 2), (7, 4), (12, 3), (4, 9), (10, 10), (14, 13)] {
                for dy in 0..3 {
                    for dx in 0..4 {
                        put(out, w, x + dx, y + dy, [0x5c, 0x5f, 0x62, 0xff]);
                    }
                }
            }
        }
        "moss_stone" => {
            noise_px(out, w, [0x7a, 0x80, 0x76, 0xff], 10, 11);
            speckle(out, w, [0x3f, 0x6b, 0x33, 0xff], 50, 12);
            speckle(out, w, [0x5a, 0x60, 0x56, 0xff], 30, 13);
        }
        "oak_log" => {
            noise_px(out, w, [0x5b, 0x45, 0x2c, 0xff], 8, 14);
            for x in [3u32, 9u32, 13u32] {
                for y in 0..w {
                    put(out, w, x, y, [0x46, 0x35, 0x21, 0xff]);
                }
            }
        }
        "oak_log_top" => {
            noise_px(out, w, [0x9a, 0x7b, 0x50, 0xff], 8, 15);
            for y in 0..w {
                for x in 0..w {
                    let d = (((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt()) as i32;
                    if d > 6 && d < 8 {
                        put(out, w, x, y, [0x6b, 0x54, 0x35, 0xff]);
                    }
                }
            }
        }
        "oak_leaves" => {
            noise_px(out, w, [0x2e, 0x5c, 0x28, 0xff], 16, 16);
            speckle(out, w, [0x46, 0x7d, 0x33, 0xff], 55, 17);
            speckle(out, w, [0x1f, 0x40, 0x1c, 0xff], 45, 18);
            speckle(out, w, [0x00, 0x00, 0x00, 0x00], 12, 19);
        }
        "water" => {
            noise_px(out, w, [0x2f, 0x54, 0x7d, 0xff], 8, 20);
            for y in [4u32, 11u32] {
                for x in 0..w {
                    put(out, w, x, y, [0x4a, 0x74, 0x9e, 0xff]);
                }
            }
        }
        "sand" => {
            noise_px(out, w, [0xd2, 0xc0, 0x8a, 0xff], 8, 21);
            speckle(out, w, [0xbd, 0xab, 0x77, 0xff], 30, 22);
        }
        "path" => {
            noise_px(out, w, [0x8a, 0x6d, 0x4c, 0xff], 10, 23);
            speckle(out, w, [0x74, 0x5a, 0x3d, 0xff], 40, 24);
        }
        "planks" => {
            noise_px(out, w, [0x9c, 0x7a, 0x4f, 0xff], 6, 25);
            for y in [0u32, 5, 10] {
                for x in 0..w {
                    put(out, w, x, y, [0x77, 0x5b, 0x3a, 0xff]);
                }
            }
            for x in [6u32] {
                for y in 1..5 {
                    put(out, w, x, y, [0x86, 0x66, 0x41, 0xff]);
                }
            }
        }
        "roof" => {
            noise_px(out, w, [0x6e, 0x3b, 0x2c, 0xff], 8, 26);
            for y in 0..w {
                let off = (y / 4) * 5;
                for x in 0..w {
                    if (x + off) % 8 == 0 {
                        put(out, w, x, y, [0x52, 0x2c, 0x21, 0xff]);
                    }
                }
            }
        }
        "dark_stone" => {
            noise_px(out, w, [0x4a, 0x4c, 0x52, 0xff], 8, 27);
            speckle(out, w, [0x39, 0x3b, 0x40, 0xff], 30, 28);
        }
        "gate_wood" => {
            noise_px(out, w, [0x77, 0x59, 0x38, 0xff], 8, 29);
            for y in 0..w {
                put(out, w, 4, y, [0x5b, 0x45, 0x2c, 0xff]);
                put(out, w, 11, y, [0x5b, 0x45, 0x2c, 0xff]);
            }
        }
        "bush" => {
            fill(out, w, [0x00, 0x00, 0x00, 0x00]);
            for y in 2..15u32 {
                for x in 2..14u32 {
                    let d = (x as f32 - 8.0).powi(2) + (y as f32 - 8.0).powi(2);
                    if d < 36.0 {
                        let v = if (x + y) % 3 == 0 { 24 } else { 0 };
                        put(
                            out,
                            w,
                            x,
                            y,
                            [0x2e + v as u8, 0x5c + v as u8, 0x28 + v as u8, 0xff],
                        );
                    }
                }
            }
        }
        "flower_white" | "flower_red" | "mushroom" => {
            fill(out, w, [0x00, 0x00, 0x00, 0x00]);
            let petal = match key {
                "flower_white" => [0xf2, 0xf2, 0xe4, 0xff],
                "flower_red" => [0xc8, 0x3a, 0x2c, 0xff],
                _ => [0xc8, 0x6a, 0x3a, 0xff],
            };
            for y in 8..16u32 {
                put(out, w, 8, y, [0x3a, 0x7a, 0x2c, 0xff]);
            }
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1), (-1, 0), (0, -1), (1, -1), (-1, 1)] {
                put(out, w, (8 + dx) as u32, (5 + dy) as u32, petal);
            }
            put(out, w, 8, 5, match key {
                "mushroom" => [0x8a, 0x5a, 0x30, 0xff],
                _ => [0xd8, 0xb2, 0x3a, 0xff],
            });
        }
        "gravel" => {
            noise_px(out, w, [0x7d, 0x78, 0x70, 0xff], 16, 30);
            speckle(out, w, [0x5d, 0x59, 0x53, 0xff], 40, 31);
        }
        "bones" => {
            fill(out, w, [0x00, 0x00, 0x00, 0x00]);
            for y in 3..13u32 {
                put(out, w, 7, y, [0xd8, 0xd2, 0xc0, 0xff]);
                put(out, w, 8, y, [0xd8, 0xd2, 0xc0, 0xff]);
            }
            for x in 5..11u32 {
                put(out, w, x, 3, [0xd8, 0xd2, 0xc0, 0xff]);
                put(out, w, x, 12, [0xd8, 0xd2, 0xc0, 0xff]);
            }
        }
        "creeper_face" => {
            fill(out, w, [0x4c, 0xa4, 0x42, 0xff]);
            speckle(out, w, [0x3f, 0x8c, 0x37, 0xff], 40, 32);
            let dark = [0x10, 0x24, 0x10, 0xff];
            for y in 3..6u32 {
                for x in [3u32, 4, 9, 10, 11] {
                    put(out, w, x, y, dark);
                }
            }
            for y in 6..9u32 {
                for x in [5u32, 6, 7, 8] {
                    put(out, w, x, y, dark);
                }
            }
            for y in 9..12u32 {
                for x in [4u32, 5, 8, 9] {
                    put(out, w, x, y, dark);
                }
            }
        }
        "skull_face" => {
            fill(out, w, [0xd4, 0xd0, 0xc0, 0xff]);
            speckle(out, w, [0xb8, 0xb4, 0xa4, 0xff], 30, 33);
            let dark = [0x22, 0x22, 0x20, 0xff];
            for y in 5..8u32 {
                for x in [3u32, 4, 10, 11] {
                    put(out, w, x, y, dark);
                }
            }
            for x in [6u32, 7, 8, 9] {
                for y in [10u32, 11] {
                    put(out, w, x, y, dark);
                }
            }
        }
        "zombie_face" => {
            fill(out, w, [0x4f, 0x7a, 0x4a, 0xff]);
            speckle(out, w, [0x44, 0x6c, 0x40, 0xff], 30, 34);
            let dark = [0x14, 0x1c, 0x14, 0xff];
            for y in [5u32, 6] {
                for x in [4u32, 5, 10, 11] {
                    put(out, w, x, y, dark);
                }
            }
            for x in 5..11u32 {
                put(out, w, x, 10, dark);
            }
        }
        "villager_face" => {
            fill(out, w, [0xa8, 0x7a, 0x54, 0xff]);
            speckle(out, w, [0x94, 0x69, 0x47, 0xff], 20, 35);
            let dark = [0x2a, 0x20, 0x16, 0xff];
            for y in [5u32, 6] {
                for x in [4u32, 5, 10, 11] {
                    put(out, w, x, y, dark);
                }
            }
            put(out, w, 8, 9, [0x6a, 0x4a, 0x30, 0xff]);
        }
        "portal" => {
            fill(out, w, [0x18, 0x10, 0x30, 0xff]);
            let mut rng = Rng(40);
            for _ in 0..60 {
                let x = rng.range(0, 15) as u32;
                let y = rng.range(0, 15) as u32;
                let c = if rng.next().is_multiple_of(2) {
                    [0x7a, 0x3a, 0xd8, 0xff]
                } else {
                    [0x4a, 0x2a, 0x90, 0xff]
                };
                put(out, w, x, y, c);
            }
        }
        "rune_stone" => {
            noise_px(out, w, [0x6a, 0x70, 0x66, 0xff], 8, 41);
            let glyph = [0x8a, 0xe8, 0xd8, 0xff];
            for y in 4..12u32 {
                put(out, w, 7, y, glyph);
                put(out, w, 8, y, glyph);
            }
            put(out, w, 6, 6, glyph);
            put(out, w, 9, 9, glyph);
        }
        _ => {
            fill(out, w, [0xff, 0x00, 0xff, 0xff]);
        }
    }
}

fn disc(px: &mut [u8], size: u32, cx: f32, cy: f32, r: f32, c: [u8; 4], feather: f32) {
    for y in 0..size {
        for x in 0..size {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            if d <= r {
                let a = if d > r - feather {
                    1.0 - (d - (r - feather)) / feather
                } else {
                    1.0
                };
                let off = ((y * size + x) * 4) as usize;
                let old = [px[off], px[off + 1], px[off + 2], px[off + 3]];
                let alpha = ((c[3] as f32) * a) as u8;
                let mut nc = c;
                nc[3] = alpha.max(old[3] / 2);
                px[off..off + 4].copy_from_slice(&nc);
            }
        }
    }
}

/// Génère une icône (24×24) procédurale pour la clé donnée.
pub fn procedural_icon(key: &str, out: &mut [u8], size: u32) {
    let s = size;
    match key {
        "heart" => {
            fill(out, s, [0, 0, 0, 0]);
            let red = [0xd8, 0x2a, 0x2a, 0xff];
            let dark = [0x2a, 0x22, 0x22, 0xff];
            disc(out, s, 8.5, 8.5, 5.2, red, 0.5);
            disc(out, s, 15.5, 8.5, 5.2, red, 0.5);
            for y in 8..19u32 {
                for x in 4..20u32 {
                    let dx = (x as f32 - 12.0).abs();
                    let dy = (y as f32 - 8.0).abs();
                    if dx + dy < 9.5 && y >= 8 {
                        put(out, s, x, y, red);
                    }
                }
            }
            disc(out, s, 9.0, 9.0, 2.0, [0xf2, 0x80, 0x80, 0xff], 0.5);
            // cadre sombre façon MCD
            for y in 2..21u32 {
                for x in 2..22u32 {
                    let edge = x == 2 || x == 21 || y == 2 || y == 20;
                    if edge {
                        put(out, s, x, y, dark);
                    }
                }
            }
        }
        "wisp" | "soul_wisp" => {
            fill(out, s, [0, 0, 0, 0]);
            disc(out, s, 12.0, 12.0, 9.0, [0x3a, 0xe8, 0x8a, 0x40], 4.0);
            disc(out, s, 12.0, 12.0, 4.5, [0x9a, 0xff, 0xd0, 0xff], 1.5);
            disc(out, s, 12.0, 12.0, 2.0, [0xff, 0xff, 0xff, 0xff], 0.5);
        }
        "arrow" => {
            fill(out, s, [0, 0, 0, 0]);
            let w = [0xc8, 0xc4, 0xb4, 0xff];
            let h = [0x8a, 0x6a, 0x44, 0xff];
            for i in 0..14u32 {
                put(out, s, 6 + i, 18 - i, w);
                put(out, s, 7 + i, 18 - i, w);
            }
            for i in 0..10u32 {
                put(out, s, 7 + i, 17 - i, h);
            }
            for (x, y) in [(4, 20), (5, 19), (4, 21), (5, 20), (6, 19)] {
                put(out, s, x, y, w);
            }
        }
        "emerald" => {
            fill(out, s, [0, 0, 0, 0]);
            let g = [0x2a, 0xd8, 0x6a, 0xff];
            let gd = [0x1a, 0x9a, 0x4a, 0xff];
            for y in 4..20u32 {
                for x in 6..18u32 {
                    let dx = (x as f32 - 11.5).abs();
                    let dy = (y as f32 - 11.5).abs();
                    if dx + dy < 7.5 {
                        put(out, s, x, y, if dx > dy { gd } else { g });
                    }
                }
            }
            disc(out, s, 9.5, 8.5, 1.6, [0xa0, 0xff, 0xc8, 0xff], 0.4);
        }
        "potion" => {
            fill(out, s, [0, 0, 0, 0]);
            let glass = [0xc8, 0xd8, 0xe8, 0xff];
            let liquid = [0xe4, 0x3a, 0x4a, 0xff];
            for y in 9..19u32 {
                for x in 7..17u32 {
                    let d = (x as f32 - 11.5).powi(2) + ((y as f32 - 13.0) * 1.15).powi(2);
                    if d < 25.0 {
                        put(out, s, x, y, if y > 12 { liquid } else { glass });
                    }
                }
            }
            for y in 5..9u32 {
                for x in 10..13u32 {
                    put(out, s, x, y, glass);
                }
            }
            for x in 9..14u32 {
                put(out, s, x, 5, [0x8a, 0x6a, 0x44, 0xff]);
            }
        }
        "totem" => {
            fill(out, s, [0, 0, 0, 0]);
            let wood = [0x9a, 0x6a, 0x3a, 0xff];
            for y in 4..20u32 {
                for x in 8..16u32 {
                    put(out, s, x, y, wood);
                }
            }
            for y in 3..8u32 {
                for x in 9..15u32 {
                    put(out, s, x, y, [0x4a, 0xd8, 0x4a, 0xff]);
                }
            }
            for y in 11..15u32 {
                for x in 10..14u32 {
                    put(out, s, x, y, [0x2a, 0xa8, 0x2a, 0xff]);
                }
            }
        }
        "beacon" => {
            fill(out, s, [0, 0, 0, 0]);
            let dark = [0x3a, 0x2a, 0x4a, 0xff];
            for y in 4..20u32 {
                for x in 7..17u32 {
                    put(out, s, x, y, dark);
                }
            }
            disc(out, s, 11.5, 11.5, 4.0, [0x8a, 0x3a, 0xd8, 0xff], 1.0);
            disc(out, s, 11.5, 11.5, 2.0, [0xd0, 0x8a, 0xff, 0xff], 0.6);
            for x in 10..13u32 {
                put(out, s, x, 3, [0x6a, 0x2a, 0xa8, 0xff]);
            }
        }
        "firework" => {
            fill(out, s, [0, 0, 0, 0]);
            let body = [0xc8, 0xc4, 0xb4, 0xff];
            for y in 8..20u32 {
                for x in 9..14u32 {
                    put(out, s, x, y, body);
                }
            }
            for x in 10..13u32 {
                put(out, s, x, 6, [0xd8, 0x3a, 0x3a, 0xff]);
            }
            for (x, y, c) in [
                (7, 4, [0xff, 0xd0, 0x4a, 0xff]),
                (11, 3, [0xff, 0x8a, 0x3a, 0xff]),
                (15, 4, [0xff, 0xd0, 0x4a, 0xff]),
                (6, 8, [0xff, 0xd0, 0x4a, 0xff]),
                (16, 8, [0xff, 0xd0, 0x4a, 0xff]),
            ] {
                put(out, s, x, y, c);
            }
        }
        "sword" => {
            fill(out, s, [0, 0, 0, 0]);
            let blade = [0xd8, 0xe0, 0xe8, 0xff];
            for i in 0..13u32 {
                put(out, s, 17 - i, 5 + i, blade);
                put(out, s, 18 - i, 5 + i, [0x9a, 0xa8, 0xb8, 0xff]);
            }
            for x in 12..18u32 {
                put(out, s, x, 16, [0x6a, 0x4a, 0x2a, 0xff]);
                put(out, s, x, 17, [0x6a, 0x4a, 0x2a, 0xff]);
            }
            for y in 17..21u32 {
                put(out, s, 11, y, [0x4a, 0x34, 0x1e, 0xff]);
                put(out, s, 12, y, [0x4a, 0x34, 0x1e, 0xff]);
            }
        }
        "bow" => {
            fill(out, s, [0, 0, 0, 0]);
            let wood = [0x8a, 0x6a, 0x3a, 0xff];
            for i in 0..14u32 {
                let ang = (i as f32 / 13.0) * std::f32::consts::PI - std::f32::consts::FRAC_PI_2;
                let x = (12.0 + 7.0 * ang.cos()) as u32;
                let y = (12.0 + 7.0 * ang.sin()) as u32;
                put(out, s, x, y, wood);
                put(out, s, x + 1, y, wood);
            }
            for i in 0..12u32 {
                put(out, s, 19, 6 + i, [0xd8, 0xd4, 0xc4, 0xff]);
            }
        }
        "armor" => {
            fill(out, s, [0, 0, 0, 0]);
            let iron = [0xb8, 0xc0, 0xc8, 0xff];
            for y in 5..9u32 {
                for x in 5..19u32 {
                    put(out, s, x, y, iron);
                }
            }
            for y in 9..19u32 {
                for x in 8..16u32 {
                    put(out, s, x, y, iron);
                }
            }
            for x in 5..8u32 {
                for y in 9..13u32 {
                    put(out, s, x, y, [0x8a, 0x94, 0x9e, 0xff]);
                }
                for y in 9..13u32 {
                    put(out, s, x + 11, y, [0x8a, 0x94, 0x9e, 0xff]);
                }
            }
        }
        "map_player" => {
            fill(out, s, [0, 0, 0, 0]);
            let w = [0x4a, 0xd8, 0xff, 0xff];
            for i in 0..10u32 {
                put(out, s, 7 + i, 6 + i, w);
                put(out, s, 8 + i, 6 + i, w);
                put(out, s, 17 - i, 6 + i, w);
                put(out, s, 16 - i, 6 + i, w);
            }
        }
        "map_portal" | "map_boss" | "map_chest" | "map_rune" | "map_villager" => {
            fill(out, s, [0, 0, 0, 0]);
            let c: [u8; 4] = match key {
                "map_portal" => [0x8a, 0x3a, 0xd8, 0xff],
                "map_boss" => [0xd8, 0x2a, 0x2a, 0xff],
                "map_chest" => [0xd8, 0xb2, 0x3a, 0xff],
                "map_rune" => [0x3a, 0xe8, 0xd8, 0xff],
                _ => [0x4a, 0xb8, 0x4a, 0xff],
            };
            disc(out, s, 12.0, 12.0, 6.5, c, 0.8);
            disc(out, s, 12.0, 12.0, 3.0, [0x10, 0x10, 0x14, 0xc0], 0.5);
        }
        "soft_dot" => {
            fill(out, s, [0, 0, 0, 0]);
            disc(out, s, 12.0, 12.0, 11.0, [0xff, 0xff, 0xff, 0xff], 11.0);
        }
        "spark" => {
            fill(out, s, [0, 0, 0, 0]);
            disc(out, s, 12.0, 12.0, 10.0, [0xff, 0xff, 0xff, 0x50], 8.0);
            disc(out, s, 12.0, 12.0, 4.0, [0xff, 0xff, 0xff, 0xff], 1.0);
        }
        "smoke" => {
            fill(out, s, [0, 0, 0, 0]);
            disc(out, s, 12.0, 12.0, 10.0, [0x88, 0x88, 0x88, 0x90], 9.0);
        }
        "ring" => {
            fill(out, s, [0, 0, 0, 0]);
            for y in 0..s {
                for x in 0..s {
                    let d = ((x as f32 - 11.5).powi(2) + (y as f32 - 11.5).powi(2)).sqrt();
                    if (d > 8.2) && d < 10.2 {
                        put(out, s, x, y, [0xff, 0xff, 0xff, 0xd0]);
                    }
                }
            }
        }
        "flame" => {
            fill(out, s, [0, 0, 0, 0]);
            disc(out, s, 12.0, 13.0, 6.0, [0xff, 0x8a, 0x2a, 0xd0], 2.0);
            disc(out, s, 12.0, 12.0, 3.2, [0xff, 0xd8, 0x6a, 0xff], 1.0);
        }
        "bubble" => {
            fill(out, s, [0, 0, 0, 0]);
            disc(out, s, 12.0, 12.0, 9.0, [0x6a, 0xd8, 0xff, 0x60], 1.5);
            disc(out, s, 9.5, 9.5, 2.0, [0xff, 0xff, 0xff, 0xd0], 0.6);
        }
        "flash" => {
            fill(out, s, [0, 0, 0, 0]);
            for y in 0..s {
                for x in 0..s {
                    let dx = (x as f32 - 11.5).abs();
                    let dy = (y as f32 - 11.5).abs();
                    if (dx - 2.5).abs() < 2.0 || (dy - 2.5).abs() < 2.0 {
                        put(out, s, x, y, [0xff, 0xf2, 0xd0, 0xe0]);
                    }
                }
            }
        }
        "aura" => {
            fill(out, s, [0, 0, 0, 0]);
            disc(out, s, 12.0, 12.0, 11.0, [0xb0, 0x5a, 0xff, 0x50], 10.0);
            disc(out, s, 12.0, 12.0, 7.0, [0xd0, 0x8a, 0xff, 0x50], 6.0);
        }
        "white" => {
            fill(out, s, [0xff, 0xff, 0xff, 0xff]);
        }
        _ => {
            fill(out, s, [0xff, 0x00, 0xff, 0xff]);
        }
    }
}
