/// Fabrique de textures 100 % procédurales (atlas 1024×1024, tuiles 16×16).
/// Chaque tuile peut être remplacée par un PNG de l'utilisateur via
/// `assets/textures/<nom>.png` — voir README.md pour la liste des noms.
use image::RgbaImage;

use crate::rng::Rng;

pub const TILE_PX: usize = 16;
pub const GRID: usize = 64;
pub const ATLAS_PX: usize = TILE_PX * GRID;

// --- Indices de tuiles -------------------------------------------------------

pub const T_WHITE: usize = 0;
pub const T_GRASS: usize = 1;
pub const T_DIRT: usize = 2;
pub const T_PATH: usize = 3;
pub const T_STONE: usize = 4;
pub const T_MOSS: usize = 5;
pub const T_PLANKS: usize = 6;
pub const T_WATER: usize = 7;
pub const T_LAVA: usize = 8;
pub const T_BRIDGE: usize = 9;
pub const T_WALL: usize = 10;
pub const T_HOUSE: usize = 11;
pub const T_WINDOW: usize = 12;
pub const T_ROOF: usize = 13;
pub const T_LEAVES: usize = 14;
pub const T_TRUNK: usize = 15;
pub const T_TOMB: usize = 16;
pub const T_CAGE: usize = 17;
pub const T_GATE: usize = 18;
pub const T_PLATE: usize = 19;
pub const T_HERO: usize = 20;
pub const T_ZOMBIE: usize = 21;
pub const T_SKELETON: usize = 22;
pub const T_CREEPER: usize = 23;
pub const T_SPIDER: usize = 24;
pub const T_VINDICATOR: usize = 25;
pub const T_ENCHANTER: usize = 26;
pub const T_NECRO: usize = 27;
pub const T_VILLAGER: usize = 28;
pub const T_MERCHANT: usize = 29;
pub const T_ARROW: usize = 30;
pub const T_ORB: usize = 31;
pub const T_HEART: usize = 32;
pub const T_SOUL: usize = 33;
pub const T_EMERALD: usize = 34;
pub const T_APPLE: usize = 35;
pub const T_POTION: usize = 36;
pub const T_WINDHORN: usize = 37;
pub const T_FIREWORK: usize = 38;
pub const T_SOULHEAL: usize = 39;
pub const T_LIFE: usize = 40;
pub const T_SKULL: usize = 41;
pub const T_CROSSHAIR: usize = 42;
pub const T_WEB: usize = 43;
pub const T_SOFT: usize = 44;
pub const T_GLOW: usize = 45;
pub const T_FIRE: usize = 46;
pub const T_COFFRE: usize = 47;
pub const T_PARCH: usize = 48;
pub const T_MUSH: usize = 49;
pub const T_BRAZIER: usize = 50;

/// Noms d'override (tuile, fichier) documentés dans le README.
pub const OVERRIDE_NAMES: &[(usize, &str)] = &[
    (T_GRASS, "grass"), (T_DIRT, "dirt"), (T_PATH, "path"), (T_STONE, "stone"),
    (T_MOSS, "moss_stone"), (T_PLANKS, "planks"), (T_WATER, "water"), (T_LAVA, "lava"),
    (T_BRIDGE, "bridge"), (T_WALL, "stone_wall"), (T_HOUSE, "house_wall"),
    (T_WINDOW, "window"), (T_ROOF, "roof"), (T_LEAVES, "leaves"), (T_TRUNK, "trunk"),
    (T_TOMB, "tombstone"), (T_CAGE, "cage"), (T_GATE, "rune_gate"), (T_PLATE, "pressure_plate"),
    (T_HERO, "hero"), (T_ZOMBIE, "zombie"), (T_SKELETON, "skeleton"), (T_CREEPER, "creeper"),
    (T_SPIDER, "spider"), (T_VINDICATOR, "vindicator"), (T_ENCHANTER, "enchanter"),
    (T_NECRO, "necromancer"), (T_VILLAGER, "villager"), (T_MERCHANT, "merchant"),
];

struct Atlas {
    img: RgbaImage,
    rng: Rng,
}

impl Atlas {
    fn px(&mut self, tile: usize, x: usize, y: usize, c: [u8; 4]) {
        let bx = (tile % GRID) * TILE_PX + x;
        let by = (tile / GRID) * TILE_PX + y;
        self.img.put_pixel(bx as u32, by as u32, image::Rgba(c));
    }

    fn rect(&mut self, tile: usize, x0: usize, y0: usize, w: usize, h: usize, c: [u8; 4]) {
        for y in y0..(y0 + h).min(TILE_PX) {
            for x in x0..(x0 + w).min(TILE_PX) {
                self.px(tile, x, y, c);
            }
        }
    }

    /// Remplit la tuile d'un bruit issu d'une palette (rendu « pixel-art »).
    fn speckle(&mut self, tile: usize, palette: &[[u8; 3]; 4]) {
        for y in 0..TILE_PX {
            for x in 0..TILE_PX {
                let i = (self.rng.next_u64() % 100) as usize;
                let idx = if i < 45 { 0 } else if i < 72 { 1 } else if i < 92 { 2 } else { 3 };
                self.px(tile, x, y, [palette[idx][0], palette[idx][1], palette[idx][2], 255]);
            }
        }
    }
}

fn put(img: &mut RgbaImage, x: u32, y: u32, c: [u8; 4]) {
    if x < img.width() && y < img.height() {
        img.put_pixel(x, y, image::Rgba(c));
    }
}

fn build_tiles(a: &mut Atlas) {
    a.speckle(T_WHITE, &[[255; 3]; 4]);
    a.speckle(T_GRASS, &[[62, 108, 48], [74, 122, 55], [54, 96, 41], [44, 82, 35]]);
    a.speckle(T_DIRT, &[[108, 74, 47], [92, 63, 39], [122, 86, 54], [80, 54, 33]]);
    a.speckle(T_PATH, &[[125, 116, 102], [110, 101, 88], [138, 128, 114], [98, 90, 78]]);
    a.speckle(T_STONE, &[[127, 127, 127], [111, 111, 111], [141, 141, 141], [97, 97, 97]]);
    a.speckle(T_MOSS, &[[116, 121, 106], [95, 106, 84], [136, 138, 126], [74, 96, 62]]);

    // Planches horizontales.
    a.speckle(T_PLANKS, &[[154, 118, 72], [141, 107, 64], [166, 129, 80], [129, 98, 58]]);
    for y in [0usize, 5, 10, 15] {
        a.rect(T_PLANKS, 0, y, TILE_PX, 1, [96, 72, 42, 255]);
    }
    a.rect(T_PLANKS, 7, 0, 1, 5, [96, 72, 42, 255]);
    a.rect(T_PLANKS, 3, 5, 1, 5, [96, 72, 42, 255]);
    a.rect(T_PLANKS, 11, 10, 1, 5, [96, 72, 42, 255]);

    a.speckle(T_WATER, &[[38, 62, 138], [32, 54, 122], [45, 74, 155], [27, 46, 108]]);
    a.speckle(T_LAVA, &[[217, 83, 30], [240, 127, 19], [143, 47, 12], [255, 205, 63]]);
    for _ in 0..10 {
        let x = a.rng.range_usize(0, TILE_PX - 2);
        let y = a.rng.range_usize(0, TILE_PX - 1);
        a.rect(T_LAVA, x, y, 2, 1, [255, 236, 120, 255]);
    }

    // Pont : planches sombres bordées.
    a.speckle(T_BRIDGE, &[[120, 90, 55], [108, 80, 48], [131, 100, 62], [98, 72, 44]]);
    a.rect(T_BRIDGE, 0, 0, TILE_PX, 1, [70, 52, 30, 255]);
    a.rect(T_BRIDGE, 0, TILE_PX - 1, TILE_PX, 1, [70, 52, 30, 255]);

    // Mur de pierre : rangées de briques décalées.
    a.speckle(T_WALL, &[[112, 112, 118], [100, 100, 106], [124, 124, 130], [88, 88, 94]]);
    for y in [0usize, 5, 10] {
        a.rect(T_WALL, 0, y, TILE_PX, 1, [72, 72, 78, 255]);
    }
    a.rect(T_WALL, 4, 0, 1, 5, [72, 72, 78, 255]);
    a.rect(T_WALL, 11, 5, 1, 5, [72, 72, 78, 255]);
    a.rect(T_WALL, 6, 10, 1, 5, [72, 72, 78, 255]);

    // Mur de maison : enduit clair + pans de bois.
    a.speckle(T_HOUSE, &[[186, 168, 136], [176, 158, 127], [196, 178, 146], [166, 148, 118]]);
    a.rect(T_HOUSE, 0, 0, 2, TILE_PX, [74, 52, 35, 255]);
    a.rect(T_HOUSE, TILE_PX - 2, 0, 2, TILE_PX, [74, 52, 35, 255]);
    a.rect(T_HOUSE, 0, 0, TILE_PX, 1, [74, 52, 35, 255]);

    // Fenêtre : cadre + lueur chaude.
    a.rect(T_WINDOW, 0, 0, TILE_PX, TILE_PX, [74, 52, 35, 255]);
    a.rect(T_WINDOW, 3, 3, 10, 10, [255, 196, 87, 255]);
    a.rect(T_WINDOW, 3, 3, 5, 5, [255, 228, 150, 255]);
    a.rect(T_WINDOW, 7, 3, 2, 10, [74, 52, 35, 255]);
    a.rect(T_WINDOW, 3, 7, 10, 2, [74, 52, 35, 255]);

    // Toit : bardeaux.
    a.speckle(T_ROOF, &[[107, 51, 37], [94, 44, 32], [120, 58, 42], [80, 38, 27]]);
    for y in [0usize, 4, 8, 12] {
        a.rect(T_ROOF, 0, y, TILE_PX, 1, [58, 26, 18, 255]);
    }

    a.speckle(T_LEAVES, &[[47, 92, 35], [39, 79, 29], [58, 107, 42], [31, 66, 24]]);
    for y in [2usize, 6, 10, 14] {
        a.rect(T_LEAVES, 0, y, TILE_PX, 1, [26, 56, 20, 255]);
    }
    a.speckle(T_TRUNK, &[[91, 66, 39], [74, 54, 32], [104, 78, 47], [62, 45, 27]]);

    // Pierre tombale : arrondie en haut.
    a.speckle(T_TOMB, &[[154, 154, 154], [140, 140, 140], [166, 166, 166], [126, 126, 126]]);
    a.rect(T_TOMB, 0, 0, 1, TILE_PX, [96, 96, 96, 255]);
    a.rect(T_TOMB, TILE_PX - 1, 0, 1, TILE_PX, [96, 96, 96, 255]);
    a.rect(T_TOMB, 7, 4, 2, 8, [110, 110, 110, 255]);

    // Cage : barreaux verticaux sombres.
    a.rect(T_CAGE, 0, 0, TILE_PX, TILE_PX, [32, 32, 42, 255]);
    for x in [1usize, 5, 9, 13] {
        a.rect(T_CAGE, x, 0, 2, TILE_PX, [86, 86, 100, 255]);
    }

    // Porte runique : pierre sombre + runes violettes.
    a.speckle(T_GATE, &[[63, 63, 74], [55, 55, 65], [72, 72, 84], [46, 46, 55]]);
    for i in 0..5 {
        let x = 2 + i * 3;
        a.rect(T_GATE, x, 3 + (i % 2) * 4, 2, 3, [160, 90, 216, 255]);
    }

    // Plaque de pression : dalle + bouton rouge.
    a.speckle(T_PLATE, &[[127, 127, 127], [111, 111, 111], [141, 141, 141], [97, 97, 97]]);
    a.rect(T_PLATE, 4, 4, 8, 8, [88, 88, 88, 255]);
    a.rect(T_PLATE, 6, 6, 4, 4, [178, 44, 44, 255]);

    // --- Peaux d'entités (une tuile = toute la peau du cube) ---
    let skin = |a: &mut Atlas, tile: usize, base: [u8; 3], dark: [u8; 3]| {
        a.speckle(tile, &[base, dark, base, dark]);
    };
    skin(a, T_ZOMBIE, [74, 122, 58], [60, 102, 46]);
    a.rect(T_ZOMBIE, 3, 6, 3, 3, [20, 20, 20, 255]);
    a.rect(T_ZOMBIE, 10, 6, 3, 3, [20, 20, 20, 255]);
    skin(a, T_SKELETON, [207, 201, 184], [178, 172, 155]);
    a.rect(T_SKELETON, 3, 6, 3, 3, [40, 40, 40, 255]);
    a.rect(T_SKELETON, 10, 6, 3, 3, [40, 40, 40, 255]);
    a.rect(T_SKELETON, 4, 11, 8, 1, [150, 144, 130, 255]);

    skin(a, T_CREEPER, [68, 148, 68], [52, 118, 52]);
    a.rect(T_CREEPER, 2, 3, 4, 4, [15, 15, 15, 255]);
    a.rect(T_CREEPER, 10, 3, 4, 4, [15, 15, 15, 255]);
    a.rect(T_CREEPER, 5, 7, 6, 5, [15, 15, 15, 255]);
    a.rect(T_CREEPER, 7, 10, 2, 6, [15, 15, 15, 255]);

    skin(a, T_SPIDER, [42, 33, 38], [33, 26, 30]);
    a.rect(T_SPIDER, 3, 5, 2, 2, [200, 40, 40, 255]);
    a.rect(T_SPIDER, 6, 4, 2, 2, [200, 40, 40, 255]);
    a.rect(T_SPIDER, 9, 4, 2, 2, [200, 40, 40, 255]);
    a.rect(T_SPIDER, 12, 5, 2, 2, [200, 40, 40, 255]);

    skin(a, T_VINDICATOR, [146, 148, 156], [128, 130, 138]);
    a.rect(T_VINDICATOR, 3, 5, 3, 2, [30, 30, 30, 255]);
    a.rect(T_VINDICATOR, 10, 5, 3, 2, [30, 30, 30, 255]);
    a.rect(T_VINDICATOR, 4, 3, 2, 1, [60, 45, 35, 255]);
    a.rect(T_VINDICATOR, 10, 3, 2, 1, [60, 45, 35, 255]);
    a.rect(T_VINDICATOR, 6, 11, 4, 1, [90, 60, 50, 255]);

    skin(a, T_ENCHANTER, [90, 58, 138], [76, 48, 118]);
    a.rect(T_ENCHANTER, 4, 2, 8, 3, [60, 40, 92, 255]);
    a.rect(T_ENCHANTER, 5, 5, 2, 2, [240, 240, 240, 255]);
    a.rect(T_ENCHANTER, 9, 5, 2, 2, [240, 240, 240, 255]);

    skin(a, T_NECRO, [44, 31, 74], [36, 25, 60]);
    a.rect(T_NECRO, 4, 2, 8, 3, [26, 18, 44, 255]);
    a.rect(T_NECRO, 5, 6, 2, 2, [110, 235, 255, 255]);
    a.rect(T_NECRO, 9, 6, 2, 2, [110, 235, 255, 255]);
    a.rect(T_NECRO, 6, 11, 4, 2, [140, 80, 220, 255]);

    skin(a, T_VILLAGER, [107, 74, 51], [93, 64, 44]);
    a.rect(T_VILLAGER, 3, 6, 3, 2, [30, 25, 20, 255]);
    a.rect(T_VILLAGER, 10, 6, 3, 2, [30, 25, 20, 255]);
    a.rect(T_VILLAGER, 7, 8, 2, 3, [120, 86, 60, 255]);

    skin(a, T_MERCHANT, [107, 74, 51], [93, 64, 44]);
    a.rect(T_MERCHANT, 0, 10, TILE_PX, 6, [46, 84, 130, 255]);
    a.rect(T_MERCHANT, 3, 6, 3, 2, [30, 25, 20, 255]);
    a.rect(T_MERCHANT, 10, 6, 3, 2, [30, 25, 20, 255]);
    a.rect(T_MERCHANT, 7, 8, 2, 3, [120, 86, 60, 255]);

    skin(a, T_HERO, [47, 127, 111], [40, 108, 94]);
    a.rect(T_HERO, 4, 1, 8, 4, [200, 152, 122, 255]);
    a.rect(T_HERO, 4, 1, 8, 2, [58, 44, 32, 255]);
    a.rect(T_HERO, 5, 3, 2, 1, [30, 30, 30, 255]);
    a.rect(T_HERO, 9, 3, 2, 1, [30, 30, 30, 255]);
    a.rect(T_HERO, 0, 8, 2, 8, [200, 152, 122, 255]);
    a.rect(T_HERO, 14, 8, 2, 8, [200, 152, 122, 255]);
}

fn build_icons(a: &mut Atlas) {
    // Transparent par défaut pour toutes les icônes.
    for t in [T_ARROW, T_ORB, T_HEART, T_SOUL, T_EMERALD, T_APPLE, T_POTION, T_WINDHORN,
              T_FIREWORK, T_SOULHEAL, T_LIFE, T_SKULL, T_CROSSHAIR, T_WEB, T_SOFT, T_GLOW,
              T_FIRE, T_COFFRE, T_PARCH, T_MUSH, T_BRAZIER, T_STONE] {
        a.rect(t, 0, 0, TILE_PX, TILE_PX, [0, 0, 0, 0]);
    }

    // Flèche : fût diagonal + pointe.
    for i in 0..9 {
        a.px(T_ARROW, 3 + i, 12 - i, [138, 100, 58, 255]);
        a.px(T_ARROW, 4 + i, 12 - i, [110, 78, 44, 255]);
    }
    a.rect(T_ARROW, 11, 2, 4, 4, [210, 214, 220, 255]);
    a.px(T_ARROW, 3, 11, [90, 90, 90, 255]);
    a.px(T_ARROW, 12, 6, [230, 230, 235, 255]);

    // Orbe (projectile du Nécromancien).
    for y in 0..TILE_PX {
        for x in 0..TILE_PX {
            let dx = x as f32 - 7.5;
            let dy = y as f32 - 7.5;
            let r = (dx * dx + dy * dy).sqrt();
            let al = ((1.0 - r / 8.0).clamp(0.0, 1.0) * 255.0) as u8;
            let c = if r < 3.0 { [255, 120, 240, al] } else { [150, 40, 220, al] };
            a.px(T_ORB, x, y, c);
        }
    }

    // Cœur.
    let heart_px = [
        (4u8, 3u8), (5, 3), (10, 3), (11, 3), (3, 4), (4, 4), (5, 4), (6, 4), (9, 4), (10, 4),
        (11, 4), (12, 4), (3, 5), (4, 5), (5, 5), (6, 5), (7, 5), (8, 5), (9, 5), (10, 5),
        (11, 5), (12, 5), (4, 6), (5, 6), (6, 6), (7, 6), (8, 6), (9, 6), (10, 6), (11, 6),
        (5, 7), (6, 7), (7, 7), (8, 7), (9, 7), (10, 7), (6, 8), (7, 8), (8, 8), (9, 8),
        (7, 9), (8, 9), (4, 2), (5, 2), (10, 2), (11, 2),
    ];
    for (x, y) in heart_px {
        a.px(T_HEART, x as usize, y as usize, [230, 40, 60, 255]);
    }
    for (x, y) in heart_px.iter().take(10) {
        a.px(T_HEART, *x as usize, *y as usize, [255, 120, 130, 255]);
    }

    // Orbe d'âme (jauge).
    for y in 0..TILE_PX {
        for x in 0..TILE_PX {
            let dx = x as f32 - 7.5;
            let dy = y as f32 - 7.5;
            let r = (dx * dx + dy * dy).sqrt();
            let al = ((1.0 - r / 7.5).clamp(0.0, 1.0) * 255.0) as u8;
            let c = if r < 4.0 { [140, 255, 120, al] } else { [40, 190, 90, al] };
            a.px(T_SOUL, x, y, c);
        }
    }

    // Émeraude.
    for y in 0..TILE_PX {
        for x in 0..TILE_PX {
            let d = (x as i32 - 8).abs() + (y as i32 - 8).abs();
            if d < 5 {
                let c = if d < 2 { [110, 255, 160, 255] } else { [30, 190, 90, 255] };
                a.px(T_EMERALD, x, y, c);
            }
        }
    }

    // Pomme.
    a.rect(T_APPLE, 4, 6, 8, 7, [200, 40, 40, 255]);
    a.rect(T_APPLE, 5, 7, 2, 2, [255, 120, 120, 255]);
    a.rect(T_APPLE, 7, 3, 1, 3, [90, 60, 30, 255]);
    a.rect(T_APPLE, 8, 4, 3, 2, [60, 140, 50, 255]);

    // Potion.
    a.rect(T_POTION, 6, 1, 4, 3, [170, 170, 180, 255]);
    a.rect(T_POTION, 4, 4, 8, 9, [200, 44, 56, 255]);
    a.rect(T_POTION, 5, 6, 2, 3, [255, 130, 140, 255]);
    a.rect(T_POTION, 4, 12, 8, 1, [120, 26, 36, 255]);

    // Cor du vent : corne courbe.
    for i in 0..8 {
        a.px(T_WINDHORN, 2 + i, 12 - i / 2, [190, 178, 150, 255]);
        a.px(T_WINDHORN, 2 + i, 13 - i / 2, [150, 138, 112, 255]);
    }
    a.rect(T_WINDHORN, 10, 4, 4, 4, [220, 210, 180, 255]);

    // Flèches d'artifice : fusée.
    a.rect(T_FIREWORK, 6, 4, 4, 8, [200, 60, 50, 255]);
    a.rect(T_FIREWORK, 6, 2, 4, 3, [240, 200, 80, 255]);
    a.rect(T_FIREWORK, 5, 12, 2, 3, [255, 160, 60, 255]);
    a.rect(T_FIREWORK, 9, 12, 2, 3, [255, 120, 40, 255]);

    // Guérison d'âme : croix verte sur orbe.
    for y in 0..TILE_PX {
        for x in 0..TILE_PX {
            let dx = x as f32 - 7.5;
            let dy = y as f32 - 7.5;
            let r = (dx * dx + dy * dy).sqrt();
            if r < 7.5 {
                a.px(T_SOULHEAL, x, y, [40, 160, 100, 200]);
            }
        }
    }
    a.rect(T_SOULHEAL, 7, 4, 2, 8, [220, 255, 230, 255]);
    a.rect(T_SOULHEAL, 4, 7, 8, 2, [220, 255, 230, 255]);

    // Vie : heaure gris.
    a.rect(T_LIFE, 3, 4, 10, 6, [150, 154, 162, 255]);
    a.rect(T_LIFE, 5, 8, 2, 3, [60, 60, 66, 255]);
    a.rect(T_LIFE, 9, 8, 2, 3, [60, 60, 66, 255]);
    a.rect(T_LIFE, 3, 10, 10, 2, [110, 114, 122, 255]);

    // Crâne.
    a.rect(T_SKULL, 4, 3, 8, 7, [230, 230, 220, 255]);
    a.rect(T_SKULL, 5, 5, 2, 2, [40, 40, 40, 255]);
    a.rect(T_SKULL, 9, 5, 2, 2, [40, 40, 40, 255]);
    a.rect(T_SKULL, 6, 10, 4, 1, [200, 200, 190, 255]);
    a.rect(T_SKULL, 7, 11, 2, 2, [200, 200, 190, 255]);

    // Réticule.
    for y in 0..TILE_PX {
        for x in 0..TILE_PX {
            let dx = x as f32 - 7.5;
            let dy = y as f32 - 7.5;
            let r = (dx * dx + dy * dy).sqrt();
            if r > 5.5 && r < 7.5 {
                a.px(T_CROSSHAIR, x, y, [255, 255, 255, 220]);
            }
        }
    }
    a.rect(T_CROSSHAIR, 7, 7, 2, 2, [255, 255, 255, 255]);

    // Toile d'araignée.
    for i in 0..TILE_PX {
        a.px(T_WEB, i, i, [235, 235, 235, 190]);
        a.px(T_WEB, 15 - i, i, [235, 235, 235, 190]);
    }
    a.rect(T_WEB, 7, 0, 2, TILE_PX, [235, 235, 235, 120]);
    a.rect(T_WEB, 0, 7, TILE_PX, 2, [235, 235, 235, 120]);

    // Particules radiales.
    for t in [T_SOFT, T_GLOW, T_FIRE] {
        for y in 0..TILE_PX {
            for x in 0..TILE_PX {
                let dx = x as f32 - 7.5;
                let dy = y as f32 - 7.5;
                let r = (dx * dx + dy * dy) / 8.0 * 2.0;
                let al = ((1.0 - r / 8.0).clamp(0.0, 1.0).powf(1.6) * 255.0) as u8;
                let c = match t {
                    t if t == T_SOFT => [235, 235, 235, al],
                    t if t == T_GLOW => [255, 240, 200, al],
                    _ => {
                        let w = ((1.0 - r / 8.0).clamp(0.0, 1.0) * 255.0) as u8;
                        [255, 120 + (w / 3), 30, al]
                    }
                };
                a.px(t, x, y, c);
            }
        }
    }

    // Coffre.
    a.rect(T_COFFRE, 1, 4, 14, 10, [138, 100, 55, 255]);
    a.rect(T_COFFRE, 1, 4, 14, 3, [110, 78, 42, 255]);
    a.rect(T_COFFRE, 7, 6, 2, 4, [220, 190, 80, 255]);

    // Parchemin.
    a.rect(T_PARCH, 3, 2, 10, 12, [222, 202, 150, 255]);
    a.rect(T_PARCH, 5, 5, 6, 1, [120, 100, 70, 255]);
    a.rect(T_PARCH, 5, 7, 6, 1, [120, 100, 70, 255]);
    a.rect(T_PARCH, 5, 9, 4, 1, [120, 100, 70, 255]);

    // Champignon lumineux : chapeau doré.
    for y in 0..TILE_PX {
        for x in 0..TILE_PX {
            let dx = x as f32 - 7.5;
            let dy = y as f32 - 7.5;
            let r = (dx * dx + dy * dy).sqrt();
            if r < 7.5 {
                let c = if r < 3.0 { [255, 240, 170, 255] } else { [240, 190, 70, 255] };
                a.px(T_MUSH, x, y, c);
            }
        }
    }

    // Braséro : flamme.
    for y in 0..TILE_PX {
        for x in 0..TILE_PX {
            let dx = x as f32 - 7.5;
            let dy = (y as f32 - 8.5) * 1.3;
            let r = (dx * dx + dy * dy).sqrt();
            let al = ((1.0 - r / 8.0).clamp(0.0, 1.0) * 255.0) as u8;
            a.px(T_BRAZIER, x, y, [255, 140 + al / 4, 40, al]);
        }
    }
}

/// Construit l'atlas complet (1024×1024 RGBA) puis applique les overrides
/// `assets/textures/<nom>.png` redimensionnés en 16×16.
pub fn build_atlas() -> RgbaImage {
    let mut a = Atlas {
        img: RgbaImage::new(ATLAS_PX as u32, ATLAS_PX as u32),
        rng: Rng::new(0x4D43_4420_2020_0C0D),
    };
    build_tiles(&mut a);
    build_icons(&mut a);
    a.img
}

fn blit_tile(img: &mut RgbaImage, tile: usize, small: &RgbaImage) {
    let bx = ((tile % GRID) * TILE_PX) as u32;
    let by = ((tile / GRID) * TILE_PX) as u32;
    for y in 0..TILE_PX as u32 {
        for x in 0..TILE_PX as u32 {
            let sx = x * small.width() / TILE_PX as u32;
            let sy = y * small.height() / TILE_PX as u32;
            let c = small.get_pixel(sx, sy);
            img.put_pixel(bx + x, by + y, *c);
        }
    }
}

pub fn apply_overrides(img: &mut RgbaImage, dir: &std::path::Path) {
    for (tile, name) in OVERRIDE_NAMES {
        let path = dir.join(format!("{name}.png"));
        let Ok(file) = image::ImageReader::open(&path) else { continue };
        let Ok(decoded) = file.decode() else {
            eprintln!("assets/textures/{name}.png : PNG illisible, tuile procédurale conservée.");
            continue;
        };
        blit_tile(img, *tile, &decoded.to_rgba8());
    }
}

/// Vignette 256×256 (assombrit les bords, teinte rouge au centre — flash dégâts géré à part).
pub fn build_vignette() -> RgbaImage {
    let sz = 256u32;
    let mut img = RgbaImage::new(sz, sz);
    for y in 0..sz {
        for x in 0..sz {
            let dx = (x as f32 / (sz - 1) as f32 - 0.5) * 2.0;
            let dy = (y as f32 / (sz - 1) as f32 - 0.5) * 2.0;
            let r = (dx * dx + dy * dy).sqrt();
            let a = (((r - 0.55) / 0.75).clamp(0.0, 1.0) * 190.0) as u8;
            put(&mut img, x, y, [8, 8, 18, a]);
        }
    }
    img
}

/// Icône de fenêtre : visage de creeper 32×32.
pub fn build_window_icon() -> Vec<u8> {
    let mut img = RgbaImage::new(32, 32);
    let mut rng = Rng::new(0xCAFE);
    for y in 0..32u32 {
        for x in 0..32u32 {
            let v = rng.next_u64() % 100;
            let c = if v < 50 { [64, 148, 64, 255] } else if v < 80 { [52, 118, 52, 255] } else { [78, 168, 78, 255] };
            put(&mut img, x, y, c);
        }
    }
    let rects = [(6u32, 6u32, 7, 7), (19, 6, 7, 7), (12, 13, 8, 7), (14, 20, 4, 9)];
    for (x, y, w, h) in rects {
        for yy in y..y + h {
            for xx in x..x + w {
                put(&mut img, xx, yy, [12, 12, 12, 255]);
            }
        }
    }
    img.into_raw()
}
