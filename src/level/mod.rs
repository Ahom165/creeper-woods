/// Mission « Creeper Woods » : carte 240×240 tuiles (2 m), 10 zones disposées
/// comme dans le jeu (lac → bois → auberge → grotte aux araignées → salle des
/// creepers → rivière → village → cimetière → tombe antique → pont de lave/temple).
/// Génération déterministe (seed fixe), maillage par chunks 16×16 pour le culling,
/// props en cubes pré-calculés, remplaçables par des modèles .obj (assets/models).
use glam::{Mat4, Vec3};
use std::f32::consts::TAU;
use image::RgbaImage;

use crate::gfx::textures;
use crate::gfx::{BillRaw, InstRaw, TerrainVertex};
use crate::obj::{load_obj, ObjMesh};
use crate::rng::Rng;

pub const TILE: f32 = 2.0;
pub const MAP: usize = 240;
pub const CHUNK: usize = 16;
pub const CHUNKS: usize = MAP / CHUNK;

// --- Tuiles -------------------------------------------------------------------

pub const G_GRASS: u8 = 0;
pub const G_DIRT: u8 = 1;
pub const G_PATH: u8 = 2;
pub const G_STONE: u8 = 3;
pub const G_MOSS: u8 = 4;
pub const G_PLANKS: u8 = 5;
pub const G_WATER: u8 = 6;
pub const G_LAVA: u8 = 7;
pub const G_BRIDGE: u8 = 8;
pub const G_WALL: u8 = 9;
pub const G_CRYPT: u8 = 10;

// --- Types de props -----------------------------------------------------------

pub const P_TREE1: u8 = 0;
pub const P_TREE2: u8 = 1;
pub const P_ROCK: u8 = 2;
pub const P_MUSH: u8 = 3;
pub const P_HOUSE: u8 = 4;
pub const P_TOMB: u8 = 5;
pub const P_BRAZIER: u8 = 6;
pub const P_STATUE: u8 = 7;
pub const P_CARAVAN: u8 = 8;
pub const P_PORTAL: u8 = 9;
pub const P_COUNT: u8 = 10;

/// Noms de fichiers attendus dans `assets/models/` (sans extension).
pub const MODEL_NAMES: [&str; P_COUNT as usize] = [
    "tree1", "tree2", "rock", "mushroom", "house", "tombstone", "brazier", "statue",
    "caravan", "portal",
];

// --- Ennemis (codes partagés avec le module de jeu) ---------------------------

pub const K_ZOMBIE: u8 = 0;
pub const K_SKELETON: u8 = 1;
pub const K_CREEPER: u8 = 2;
pub const K_SPIDER: u8 = 3;
pub const K_VINDICATOR: u8 = 4;
pub const K_ENCHANTER: u8 = 5;
pub const K_NECRO: u8 = 6;

// --- Interactables ------------------------------------------------------------

pub const I_VILLAGER: u8 = 0;
pub const I_MERCHANT: u8 = 1;
pub const I_CHEST: u8 = 2;
pub const I_BUTTON: u8 = 3;
pub const I_PLATE: u8 = 4;
pub const I_PARCH: u8 = 5;
pub const I_RUNEGATE: u8 = 6;
pub const I_EXIT: u8 = 7;
pub const I_BOSS: u8 = 8;

// --- Zones ---------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct ZoneDef {
    pub name: &'static str,
    /// [x0, y0, x1, y1] en tuiles, bornes incluses.
    pub rect: [u16; 4],
    /// Point de réapparition (tuiles).
    pub anchor: [u16; 2],
}

// Zones désignées par index dans ZONES (0 lac, 1 bois, 2 auberge, 3 grotte,
// 4 creepers, 5 rivière, 6 village, 7 cimetière, 8 tombe, 9 temple).
pub const Z_BOIS: usize = 1;
pub const Z_AUBERGE: usize = 2;
pub const Z_VILLAGE: usize = 6;
pub const Z_TOMBE: usize = 8;
pub const Z_TEMPLE: usize = 9;

pub const ZONES: [ZoneDef; 10] = [
    ZoneDef { name: "Lac aux têtes de pierre", rect: [16, 114, 64, 164], anchor: [40, 140] },
    ZoneDef { name: "Bois profonds", rect: [36, 60, 112, 128], anchor: [74, 96] },
    ZoneDef { name: "Auberge", rect: [76, 30, 118, 66], anchor: [96, 48] },
    ZoneDef { name: "Grotte aux araignées", rect: [104, 18, 150, 60], anchor: [128, 40] },
    ZoneDef { name: "Salle des creepers", rect: [144, 20, 178, 54], anchor: [162, 38] },
    ZoneDef { name: "Rivière aux ponts", rect: [166, 52, 214, 110], anchor: [190, 80] },
    ZoneDef { name: "Village encerclé", rect: [82, 104, 152, 162], anchor: [116, 134] },
    ZoneDef { name: "Cimetière", rect: [150, 100, 206, 154], anchor: [178, 128] },
    ZoneDef { name: "Tombe antique", rect: [52, 166, 112, 214], anchor: [82, 190] },
    ZoneDef { name: "Pont de lave & temple", rect: [106, 162, 198, 230], anchor: [150, 196] },
];

#[derive(Clone)]
pub struct Prop {
    pub kind: u8,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub radius: f32,
    /// Parties procédurales pré-calculées (cubes).
    pub parts: std::rc::Rc<Vec<InstRaw>>,
    /// Index dans `models` si un .obj remplace ce prop.
    pub model: Option<u32>,
}

/// Modèle .obj chargé : géométrie + texture dédiée (facultative).
pub struct LoadedModel {
    pub mesh: ObjMesh,
    pub tex: Option<RgbaImage>,
}

#[derive(Clone, Copy)]
pub struct SpawnDef {
    pub kind: u8,
    pub x: f32,
    pub z: f32,
}

#[derive(Clone, Copy)]
pub struct InterDef {
    pub kind: u8,
    pub x: f32,
    pub z: f32,
    pub data: u8,
}

#[derive(Clone, Copy)]
pub struct LightSrc {
    pub pos: Vec3,
    pub color: [f32; 3],
    pub radius: f32,
    pub intensity: f32,
}

pub struct Level {
    pub ground: Vec<u8>,
    pub solid: Vec<bool>,
    pub anim: Vec<u8>,
    pub props: Vec<Prop>,
    /// Index de props par chunk (collisions cercle + culling).
    pub prop_grid: Vec<Vec<u32>>,
    pub static_bills: Vec<(Vec3, BillRaw)>,
    pub lights: Vec<LightSrc>,
    pub spawners: Vec<SpawnDef>,
    pub inters: Vec<InterDef>,
    pub models: Vec<LoadedModel>,
    /// Par type de prop : index du modèle .obj chargé, sinon rien.
    pub pending_model: [Option<u32>; P_COUNT as usize],
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
    /// Par chunk : (premier index, nb d'indices, centre, rayon) pour le culling.
    pub chunk_draws: Vec<(u32, u32, Vec3, f32)>,
    pub map: RgbaImage,
    pub spawn: [f32; 2],
    /// Rect (tuiles) du déclencheur « Trouvez la caravane ».
    pub caravan_rect: [u16; 4],
}

impl Level {
    pub fn build() -> Self {
        let mut lv = Level {
            ground: vec![G_GRASS; MAP * MAP],
            solid: vec![false; MAP * MAP],
            anim: vec![0; MAP * MAP],
            props: Vec::new(),
            prop_grid: vec![Vec::new(); CHUNKS * CHUNKS],
            static_bills: Vec::new(),
            lights: Vec::new(),
            spawners: Vec::new(),
            inters: Vec::new(),
            models: Vec::new(),
            pending_model: [None; P_COUNT as usize],
            vertices: Vec::new(),
            indices: Vec::new(),
            chunk_draws: Vec::new(),
            map: RgbaImage::new(MAP as u32, MAP as u32),
            spawn: [40.5 * TILE, 140.5 * TILE],
            caravan_rect: [70, 52, 92, 68],
        };
        let mut rng = Rng::new(0x2020_0520_2020_0520);

        lv.load_models();
        lv.border_walls();
        lv.fill_rect(166, 52, 214, 110, G_DIRT); // berges de la rivière
        lv.river_and_bridges();
        lv.carve_path(&[
            [40, 140], [56, 120], [74, 96], [96, 48], [128, 40], [162, 38],
            [190, 80], [150, 110], [116, 134], [178, 128], [82, 190],
            [150, 190], [150, 210],
        ]);
        lv.lava_and_bridge();
        lv.temple_platform();
        lv.crypt();
        lv.pond_and_statues();
        lv.scatter_vegetation(&mut rng);
        lv.place_houses();
        lv.place_props(&mut rng);
        lv.place_spawns();
        lv.place_inters();
        lv.bake_mesh();
        lv.build_map();
        lv
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * MAP + x
    }

    fn set(&mut self, x: i32, y: i32, g: u8) {
        if (0..MAP as i32).contains(&x) && (0..MAP as i32).contains(&y) {
            let i = self.idx(x as usize, y as usize);
            self.ground[i] = g;
            self.anim[i] = match g {
                G_WATER => 1,
                G_LAVA => 2,
                _ => 0,
            };
            self.solid[i] = matches!(g, G_WALL | G_WATER | G_LAVA);
        }
    }

    pub fn tile(&self, x: usize, y: usize) -> u8 {
        self.ground[self.idx(x.min(MAP - 1), y.min(MAP - 1))]
    }

    pub fn solid_at(&self, wx: f32, wz: f32) -> bool {
        let tx = (wx / TILE).floor() as isize;
        let ty = (wz / TILE).floor() as isize;
        if tx < 0 || ty < 0 || tx as usize >= MAP || ty as usize >= MAP {
            return true;
        }
        self.solid[self.idx(tx as usize, ty as usize)]
    }

    pub fn ground_at(&self, wx: f32, wz: f32) -> u8 {
        let tx = (wx / TILE).floor() as isize;
        let ty = (wz / TILE).floor() as isize;
        if tx < 0 || ty < 0 || tx as usize >= MAP || ty as usize >= MAP {
            return G_WALL;
        }
        self.ground[self.idx(tx as usize, ty as usize)]
    }

    pub fn zone_at(&self, wx: f32, wz: f32) -> Option<usize> {
        let tx = (wx / TILE) as u16;
        let ty = (wz / TILE) as u16;
        ZONES
            .iter()
            .position(|z| tx >= z.rect[0] && ty >= z.rect[1] && tx <= z.rect[2] && ty <= z.rect[3])
    }

    /// Cercles de collision des props proches (troncs, rochers…).
    pub fn circles_near(&self, wx: f32, wz: f32) -> Vec<(f32, f32, f32)> {
        let cx = ((wx / TILE) as usize / CHUNK).min(CHUNKS - 1);
        let cy = ((wz / TILE) as usize / CHUNK).min(CHUNKS - 1);
        let mut out = Vec::new();
        for j in cy.saturating_sub(1)..=(cy + 1).min(CHUNKS - 1) {
            for i in cx.saturating_sub(1)..=(cx + 1).min(CHUNKS - 1) {
                for &pi in &self.prop_grid[j * CHUNKS + i] {
                    let p = &self.props[pi as usize];
                    if p.radius > 0.0 {
                        out.push((p.x, p.z, p.radius));
                    }
                }
            }
        }
        out
    }

    /// Charge `assets/models/<nom>.obj` (+ `<nom>.png` facultatif) pour chaque
    /// type de prop. À défaut, le prop garde ses cubes procéduraux.
    fn load_models(&mut self) {
        for (kind, name) in MODEL_NAMES.iter().enumerate() {
            let path = format!("assets/models/{name}.obj");
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Some(mesh) = load_obj(&text) else {
                eprintln!("assets/models/{name}.obj : format non reconnu, cubes procéduraux conservés.");
                continue;
            };
            let tex = image::ImageReader::open(format!("assets/models/{name}.png"))
                .ok()
                .and_then(|r| r.decode().ok())
                .map(|d| d.to_rgba8());
            // Marque tous les props futurs de ce type : les props sont ajoutés
            // après, on retient l'index par kind.
            let id = self.models.len() as u32;
            self.models.push(LoadedModel { mesh, tex });
            self.pending_model[kind] = Some(id);
        }
    }

    fn border_walls(&mut self) {
        for y in 0..MAP {
            for x in 0..MAP {
                if x < 3 || y < 3 || x >= MAP - 3 || y >= MAP - 3 {
                    self.set(x as i32, y as i32, G_WALL);
                }
            }
        }
    }

    fn fill_rect(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, g: u8) {
        for y in y0..=(y1.min(MAP as u16 - 4)) {
            for x in x0..=(x1.min(MAP as u16 - 4)) {
                self.set(x as i32, y as i32, g);
            }
        }
    }

    /// Couloir en L de 5 tuiles de large entre waypoints ; ne marque PATH que
    /// sur l'herbe/la terre (l'eau, la lave et le temple sont préservés).
    fn carve_path(&mut self, wps: &[[u16; 2]]) {
        for w in wps.windows(2) {
            let [ax, ay] = w[0];
            let [bx, by] = w[1];
            let (x0, x1) = (ax.min(bx), ax.max(bx));
            let (y0, y1) = (ay.min(by), ay.max(by));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let on_arm1 = x >= ax.saturating_sub(2) && x <= ax + 2 && y >= y0 && y <= y1;
                    let on_arm2 = y >= by.saturating_sub(2) && y <= by + 2 && x >= x0 && x <= x1;
                    if on_arm1 || on_arm2 {
                        let g = self.tile(x as usize, y as usize);
                        if matches!(g, G_GRASS | G_DIRT) {
                            self.set(x as i32, y as i32, G_PATH);
                        }
                    }
                }
            }
        }
    }

    fn river_and_bridges(&mut self) {
        for y in 52..=110 {
            for x in 186..=192 {
                self.set(x, y, G_WATER);
            }
        }
        for (by0, by1) in [(62i32, 66), (88, 92)] {
            for y in by0..=by1 {
                for x in 183..=195 {
                    self.set(x, y, G_BRIDGE);
                }
            }
        }
    }

    fn lava_and_bridge(&mut self) {
        for y in 186..=192 {
            for x in 116..=188 {
                self.set(x, y, G_LAVA);
            }
        }
        for x in 115..=189 {
            self.set(x, 185, G_STONE);
            self.set(x, 193, G_STONE);
        }
        for y in 184..=194 {
            for x in 147..=153 {
                self.set(x, y, G_BRIDGE);
            }
        }
    }

    fn temple_platform(&mut self) {
        self.fill_rect(132, 200, 170, 224, G_STONE);
        for y in 202..=222 {
            for x in 134..=168 {
                if (x + y) % 7 != 0 {
                    self.set(x, y, G_MOSS);
                }
            }
        }
        for y in 194..=224 {
            for x in 148..=152 {
                self.set(x, y, G_PLANKS);
            }
        }
    }

    fn crypt(&mut self) {
        // Salle secrète derrière le cimetière, ouverture scellée par la porte
        // runique actionnée par la plaque de pression.
        for y in 138..=150 {
            for x in 188..=202 {
                if x == 188 || x == 202 || y == 138 || y == 150 {
                    self.set(x, y, G_WALL);
                } else {
                    self.set(x, y, G_MOSS);
                }
            }
        }
        for x in 193..=197 {
            self.set(x, 151, G_STONE);
            self.set(x, 152, G_PATH);
            self.set(x, 153, G_PATH);
        }
        for x in 194..=196 {
            self.set(x, 150, G_CRYPT);
        }
    }

    fn pond_and_statues(&mut self) {
        for y in 146..=154 {
            for x in 26..=36 {
                self.set(x, y, G_WATER);
            }
        }
    }

    fn scatter_vegetation(&mut self, rng: &mut Rng) {
        let mut tries = 5200usize;
        while tries > 0 {
            tries -= 1;
            let x = rng.range_usize(4, MAP - 4);
            let y = rng.range_usize(4, MAP - 4);
            let g = self.tile(x, y);
            if g != G_GRASS && g != G_DIRT {
                continue;
            }
            let zone = self.zone_at(x as f32 * TILE, y as f32 * TILE);
            let dense = zone == Some(Z_BOIS);
            let p = if dense { 0.40 } else { 0.09 };
            if !rng.chance(p) {
                continue;
            }
            let kind = if rng.chance(0.7) { P_TREE1 } else { P_TREE2 };
            self.add_prop(kind, x as f32 * TILE, y as f32 * TILE, rng.range_f32(0.0, TAU), 1.0);
        }
        for _ in 0..260 {
            let x = rng.range_usize(4, MAP - 4);
            let y = rng.range_usize(4, MAP - 4);
            if self.tile(x, y) == G_GRASS && rng.chance(0.5) {
                self.add_prop(P_ROCK, x as f32 * TILE, y as f32 * TILE, rng.range_f32(0.0, TAU), 1.0);
            }
        }
        for _ in 0..170 {
            let x = rng.range_usize(6, MAP - 6);
            let y = rng.range_usize(6, MAP - 6);
            let g = self.tile(x, y);
            if (g == G_GRASS || g == G_DIRT) && rng.chance(0.55) {
                self.add_prop(P_MUSH, x as f32 * TILE + 0.6, y as f32 * TILE + 0.6, 0.0, 1.0);
            }
        }
    }

    fn place_houses(&mut self) {
        for (hx, hy) in [(84usize, 40usize), (104usize, 34usize), (100usize, 56usize)] {
            self.add_house(hx, hy);
        }
        for (hx, hy) in [(92usize, 116usize), (110, 112), (134, 116), (96, 148), (126, 152), (142, 140)] {
            self.add_house(hx, hy);
        }
    }

    fn add_house(&mut self, tx: usize, ty: usize) {
        for y in ty..ty + 3 {
            for x in tx..tx + 4 {
                let i = self.idx(x, y);
                self.solid[i] = true;
            }
        }
        self.add_prop(P_HOUSE, tx as f32 * TILE + 4.0, ty as f32 * TILE + 3.0, 0.0, 1.0);
    }

    fn place_props(&mut self, rng: &mut Rng) {
        // Statues du lac aux têtes de pierre.
        for (sx, sz) in [(24.0, 143.0), (38.0, 143.0), (31.0, 156.0)] {
            self.add_prop(P_STATUE, sx * TILE, sz * TILE, 0.0, 1.0);
        }
        // Toiles d'araignée dans la grotte (billboards statiques).
        for _ in 0..26 {
            let tx = rng.range_usize(108, 146);
            let ty = rng.range_usize(22, 56);
            if self.tile(tx, ty) != G_WALL && self.tile(tx, ty) != G_WATER {
                let x = tx as f32 * TILE;
                let z = ty as f32 * TILE;
                self.static_bills.push((
                    Vec3::new(x, 1.5, z),
                    BillRaw {
                        pos_size: [x, 1.5, z, 2.4],
                        color: [1.0, 1.0, 1.0, 0.5],
                        params: [0.0, textures::T_WEB as f32, 0.0, 0.0],
                    },
                ));
            }
        }
        // Têtes de creeper sculptées dans la salle des creepers.
        for (sx, sz) in [(150.0, 26.0), (166.0, 30.0), (158.0, 44.0), (172.0, 46.0)] {
            self.add_prop(P_STATUE, sx * TILE, sz * TILE, rng.range_f32(0.0, TAU), 1.0);
        }
        // Tombes du cimetière.
        for y in (104..=150).step_by(4) {
            for x in (154..=200).step_by(5) {
                let g = self.tile(x, y);
                if g == G_GRASS || g == G_DIRT {
                    self.add_prop(P_TOMB, x as f32 * TILE + 1.0, y as f32 * TILE + 1.0, 0.0, 1.0);
                }
            }
        }
        // Braséros du temple.
        for (bx, bz) in [(136.0, 204.0), (166.0, 204.0), (136.0, 220.0), (166.0, 220.0)] {
            self.add_prop(P_BRAZIER, bx * TILE, bz * TILE, 0.0, 1.0);
        }
        // Caravane abandonnée (objectif « Trouvez la caravane »).
        self.add_prop(P_CARAVAN, 78.0 * TILE, 60.0 * TILE, 0.35, 1.0);
        // Portail de sortie du temple.
        self.add_prop(P_PORTAL, 150.0 * TILE, 224.0 * TILE, 0.0, 1.0);
    }

    fn add_prop(&mut self, kind: u8, x: f32, z: f32, yaw: f32, scale: f32) {
        let parts = bake_prop(kind, x, z, yaw, scale);
        let model = self.pending_model[kind as usize];
        let radius = if let Some(mid) = model {
            // Rayon de collision déduit de l'encombrement du modèle .obj.
            let mesh = &self.models[mid as usize].mesh;
            let r = mesh
                .verts
                .iter()
                .map(|v| (v.pos[0] * v.pos[0] + v.pos[2] * v.pos[2]).sqrt())
                .fold(0.0_f32, f32::max);
            r.max(0.4)
        } else {
            match kind {
                P_TREE1 | P_TREE2 => 0.55,
                P_ROCK => 0.9,
                P_MUSH => 0.25,
                P_HOUSE | P_CARAVAN | P_PORTAL => 0.0,
                P_TOMB => 0.45,
                P_BRAZIER => 0.5,
                P_STATUE => 1.1,
                _ => 0.5,
            }
        };
        let prop = Prop { kind, x, z, yaw, radius, parts: std::rc::Rc::new(parts), model };
        let cx = ((x / TILE) as usize / CHUNK).min(CHUNKS - 1);
        let cy = ((z / TILE) as usize / CHUNK).min(CHUNKS - 1);
        self.prop_grid[cy * CHUNKS + cx].push(self.props.len() as u32);
        self.props.push(prop);

        match kind {
            P_MUSH => {
                self.lights.push(LightSrc {
                    pos: Vec3::new(x, 1.3, z),
                    color: [1.0, 0.85, 0.35],
                    radius: 8.0,
                    intensity: 1.1,
                });
                self.static_bills.push((
                    Vec3::new(x, 1.25, z),
                    BillRaw {
                        pos_size: [x, 1.25, z, 2.0],
                        color: [1.0, 0.85, 0.4, 0.35],
                        params: [0.0, textures::T_GLOW as f32, 0.0, 0.0],
                    },
                ));
            }
            P_BRAZIER => {
                self.lights.push(LightSrc {
                    pos: Vec3::new(x, 1.5, z),
                    color: [1.0, 0.55, 0.2],
                    radius: 10.0,
                    intensity: 1.4,
                });
                self.static_bills.push((
                    Vec3::new(x, 1.55, z),
                    BillRaw {
                        pos_size: [x, 1.55, z, 1.4],
                        color: [1.0, 0.8, 0.4, 0.85],
                        params: [0.0, textures::T_BRAZIER as f32, 0.0, 0.0],
                    },
                ));
            }
            P_HOUSE => self.lights.push(LightSrc {
                pos: Vec3::new(x, 2.2, z + 3.4),
                color: [1.0, 0.75, 0.4],
                radius: 9.0,
                intensity: 0.9,
            }),
            P_PORTAL => {
                self.lights.push(LightSrc {
                    pos: Vec3::new(x, 1.8, z),
                    color: [0.75, 0.4, 1.0],
                    radius: 12.0,
                    intensity: 1.6,
                });
                self.static_bills.push((
                    Vec3::new(x, 1.8, z),
                    BillRaw {
                        pos_size: [x, 1.8, z, 2.8],
                        color: [0.7, 0.4, 1.0, 0.75],
                        params: [0.0, textures::T_ORB as f32, 0.0, 0.0],
                    },
                ));
            }
            _ => {}
        }
    }

    fn place_spawns(&mut self) {
        let push = |lv: &mut Self, kind: u8, pts: &[[i32; 2]]| {
            for pt in pts {
                lv.spawners.push(SpawnDef {
                    kind,
                    x: pt[0] as f32 * TILE,
                    z: pt[1] as f32 * TILE,
                });
            }
        };
        push(self, K_ZOMBIE, &[
            [54, 122], [62, 112], [70, 104], [80, 96],
            [66, 90], [86, 84], [58, 78], [92, 70],
        ]);
        push(self, K_SKELETON, &[[60, 116], [76, 100], [84, 88], [90, 74]]);
        push(self, K_ZOMBIE, &[[88, 54], [96, 44], [106, 50], [110, 40]]);
        push(self, K_SKELETON, &[[92, 34], [104, 30]]);
        push(self, K_SPIDER, &[
            [112, 30], [120, 26], [128, 32], [136, 26],
            [142, 34], [124, 44], [134, 50], [144, 46],
        ]);
        push(self, K_CREEPER, &[[152, 28], [160, 34], [168, 28], [156, 46], [170, 40], [162, 50]]);
        push(self, K_SKELETON, &[[196, 64], [200, 76], [196, 92], [204, 100]]);
        push(self, K_ZOMBIE, &[[198, 70], [202, 88]]);
        push(self, K_ZOMBIE, &[
            [92, 128], [100, 140], [112, 124], [124, 146],
            [136, 128], [108, 156], [146, 148],
        ]);
        push(self, K_SKELETON, &[[104, 118], [130, 138], [118, 154]]);
        push(self, K_ENCHANTER, &[[120, 136]]);
        push(self, K_ZOMBIE, &[[158, 108], [166, 120], [176, 112], [186, 122], [196, 112]]);
        push(self, K_SKELETON, &[[162, 132], [180, 140], [192, 128]]);
        push(self, K_VINDICATOR, &[[172, 126], [190, 138]]);
        push(self, K_ENCHANTER, &[[180, 118]]);
        push(self, K_ZOMBIE, &[[64, 182], [74, 196], [88, 204], [70, 206], [96, 184]]);
        push(self, K_VINDICATOR, &[[82, 196]]);
        push(self, K_ZOMBIE, &[[124, 172], [136, 178], [158, 172], [170, 180]]);
        push(self, K_SKELETON, &[[130, 166], [162, 168]]);
        self.spawners.push(SpawnDef { kind: K_NECRO, x: 150.0 * TILE, z: 212.0 * TILE });
    }

    fn place_inters(&mut self) {
        // Villageois de l'introduction, au bord du lac.
        self.inters.push(InterDef { kind: I_VILLAGER, x: 44.5 * TILE, z: 132.5 * TILE, data: 0 });
        // Marchand capturé dans la grotte aux araignées.
        self.inters.push(InterDef { kind: I_MERCHANT, x: 132.5 * TILE, z: 44.5 * TILE, data: 0 });
        // Coffres de ravitaillement : auberge + caravane.
        self.inters.push(InterDef { kind: I_CHEST, x: 94.0 * TILE, z: 46.0 * TILE, data: 0 });
        self.inters.push(InterDef { kind: I_CHEST, x: 75.5 * TILE, z: 57.0 * TILE, data: 1 });
        // Cinq villageois du village encerclé.
        for (i, (_idx, cx, cz)) in [(0u16, 96u16, 124u16), (1, 108, 146), (2, 124, 118), (3, 134, 150), (4, 116, 138)]
            .iter()
            .enumerate()
        {
            self.inters.push(InterDef {
                kind: I_VILLAGER,
                x: (*cx) as f32 * TILE + 1.0,
                z: (*cz) as f32 * TILE + 1.0,
                data: i as u8 + 1,
            });
        }
        // Bouton de la tombe antique → porte runique du temple.
        self.inters.push(InterDef { kind: I_BUTTON, x: 66.5 * TILE, z: 196.5 * TILE, data: 0 });
        self.inters.push(InterDef { kind: I_RUNEGATE, x: 118.0 * TILE, z: 190.0 * TILE, data: 0 });
        // Plaque de pression du cimetière → crypte secrète.
        self.inters.push(InterDef { kind: I_PLATE, x: 190.0 * TILE, z: 134.0 * TILE, data: 0 });
        self.inters.push(InterDef { kind: I_RUNEGATE, x: 195.0 * TILE, z: 150.5 * TILE, data: 1 });
        // Parchemin secret dans la crypte.
        self.inters.push(InterDef { kind: I_PARCH, x: 195.5 * TILE, z: 144.5 * TILE, data: 0 });
        // Sortie du temple (activée après le boss) + déclencheur du boss.
        self.inters.push(InterDef { kind: I_EXIT, x: 150.0 * TILE, z: 224.0 * TILE, data: 0 });
        self.inters.push(InterDef { kind: I_BOSS, x: 150.0 * TILE, z: 204.0 * TILE, data: 0 });
    }

    fn bake_mesh(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.chunk_draws.clear();
        for cy in 0..CHUNKS {
            for cx in 0..CHUNKS {
                let first = self.indices.len() as u32;
                for ly in 0..CHUNK {
                    for lx in 0..CHUNK {
                        self.emit_tile(cx * CHUNK + lx, cy * CHUNK + ly);
                    }
                }
                let count = self.indices.len() as u32 - first;
                let half = CHUNK as f32 * TILE * 0.5;
                let center = Vec3::new((cx * CHUNK + CHUNK / 2) as f32 * TILE, 1.0, (cy * CHUNK + CHUNK / 2) as f32 * TILE);
                self.chunk_draws.push((first, count, center, half * 1.415 + 3.0));
            }
        }
    }

    fn emit_tile(&mut self, x: usize, y: usize) {
        let g = self.tile(x, y);
        if g == G_WALL {
            let (x0, z0) = (x as f32 * TILE, y as f32 * TILE);
            let (x1, z1) = (x0 + TILE, z0 + TILE);
            let h = 2.6;
            self.quad(
                [Vec3::new(x0, h, z0), Vec3::new(x0, h, z1), Vec3::new(x1, h, z1), Vec3::new(x1, h, z0)],
                Vec3::Y,
                textures::T_MOSS,
                0.0,
                0.0,
            );
            if y + 1 < MAP && self.tile(x, y + 1) != G_WALL {
                self.quad(
                    [Vec3::new(x0, 0.0, z1), Vec3::new(x1, 0.0, z1), Vec3::new(x1, h, z1), Vec3::new(x0, h, z1)],
                    Vec3::Z,
                    textures::T_WALL,
                    0.0,
                    0.0,
                );
            }
            if y > 0 && self.tile(x, y - 1) != G_WALL {
                self.quad(
                    [Vec3::new(x1, 0.0, z0), Vec3::new(x0, 0.0, z0), Vec3::new(x0, h, z0), Vec3::new(x1, h, z0)],
                    -Vec3::Z,
                    textures::T_WALL,
                    0.0,
                    0.0,
                );
            }
            if x + 1 < MAP && self.tile(x + 1, y) != G_WALL {
                self.quad(
                    [Vec3::new(x1, 0.0, z0), Vec3::new(x1, 0.0, z1), Vec3::new(x1, h, z1), Vec3::new(x1, h, z0)],
                    Vec3::X,
                    textures::T_WALL,
                    0.0,
                    0.0,
                );
            }
            if x > 0 && self.tile(x - 1, y) != G_WALL {
                self.quad(
                    [Vec3::new(x0, 0.0, z1), Vec3::new(x0, 0.0, z0), Vec3::new(x0, h, z0), Vec3::new(x0, h, z1)],
                    -Vec3::X,
                    textures::T_WALL,
                    0.0,
                    0.0,
                );
            }
            return;
        }

        let tile = match g {
            G_GRASS => textures::T_GRASS,
            G_DIRT => textures::T_DIRT,
            G_PATH => textures::T_PATH,
            G_STONE => textures::T_STONE,
            G_MOSS => textures::T_MOSS,
            G_PLANKS => textures::T_PLANKS,
            G_WATER => textures::T_WATER,
            G_LAVA => textures::T_LAVA,
            G_BRIDGE => textures::T_BRIDGE,
            G_CRYPT => textures::T_MOSS,
            _ => textures::T_GRASS,
        };
        let emissive = f32::from(g == G_LAVA);
        let anim = self.anim[self.idx(x, y)] as f32;
        let (x0, z0) = (x as f32 * TILE, y as f32 * TILE);
        let (x1, z1) = (x0 + TILE, z0 + TILE);
        self.quad(
            [Vec3::new(x0, 0.0, z0), Vec3::new(x0, 0.0, z1), Vec3::new(x1, 0.0, z1), Vec3::new(x1, 0.0, z0)],
            Vec3::Y,
            tile,
            emissive,
            anim,
        );
    }

    /// Quad CCW vu depuis la normale `n` : (bas-gauche, haut-gauche, haut-droit,
    /// bas-droit) du plan de la face.
    fn quad(&mut self, p: [Vec3; 4], n: Vec3, tile: usize, emissive: f32, anim: f32) {
        let base = self.vertices.len() as u32;
        let u0 = (tile % textures::GRID) as f32 / textures::GRID as f32;
        let v0 = (tile / textures::GRID) as f32 / textures::GRID as f32;
        let s = 1.0 / textures::GRID as f32;
        let inset = s * 0.01;
        let uvs = [
            [u0 + inset, v0 + s - inset],
            [u0 + inset, v0 + inset],
            [u0 + s - inset, v0 + inset],
            [u0 + s - inset, v0 + s - inset],
        ];
        for (i, pos) in p.iter().enumerate() {
            self.vertices.push(TerrainVertex {
                pos: pos.to_array(),
                nrm: n.to_array(),
                uv: uvs[i],
                emissive,
                anim,
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn build_map(&mut self) {
        for y in 0..MAP {
            for x in 0..MAP {
                let c: [u8; 4] = match self.tile(x, y) {
                    G_GRASS => [74, 110, 58, 255],
                    G_DIRT => [116, 88, 60, 255],
                    G_PATH => [146, 136, 120, 255],
                    G_STONE => [126, 126, 134, 255],
                    G_MOSS => [98, 126, 92, 255],
                    G_PLANKS => [164, 128, 78, 255],
                    G_WATER => [52, 78, 152, 255],
                    G_LAVA => [255, 104, 30, 255],
                    G_BRIDGE => [150, 114, 68, 255],
                    G_WALL => [62, 62, 70, 255],
                    G_CRYPT => [84, 110, 84, 255],
                    _ => [74, 110, 58, 255],
                };
                self.map.put_pixel(x as u32, y as u32, image::Rgba(c));
            }
        }
        for p in &self.props {
            if p.kind == P_TREE1 || p.kind == P_TREE2 {
                let x = ((p.x / TILE) as u32).min(MAP as u32 - 1);
                let y = ((p.z / TILE) as u32).min(MAP as u32 - 1);
                self.map.put_pixel(x, y, image::Rgba([30, 56, 26, 255]));
            }
        }
    }
}

fn inst(m: Mat4, tile: usize, tint: [f32; 4], emissive: bool) -> InstRaw {
    let u0 = (tile % textures::GRID) as f32 / textures::GRID as f32;
    let v0 = (tile / textures::GRID) as f32 / textures::GRID as f32;
    let s = 1.0 / textures::GRID as f32;
    let inset = s * 0.02;
    InstRaw {
        model: m.to_cols_array_2d(),
        tint: [tint[0], tint[1], tint[2], f32::from(emissive)],
        uv0: [u0 + inset, v0 + inset],
        uvs: [s - 2.0 * inset, s - 2.0 * inset],
    }
}

/// Construit les cubes d'un prop (fallback procédural ; un .obj chargé s'y
/// substitue au rendu). Réutilise le pipeline des entités.
fn bake_prop(kind: u8, x: f32, z: f32, yaw: f32, scale: f32) -> Vec<InstRaw> {
    let mut parts = Vec::new();
    let box_at = |parts: &mut Vec<InstRaw>,
                      dy: f32,
                      dx: f32,
                      dz: f32,
                      sx: f32,
                      sy: f32,
                      sz: f32,
                      tile: usize,
                      tint: [f32; 4],
                      emis: bool| {
        let m = Mat4::from_translation(Vec3::new(x + dx, dy * scale, z + dz))
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_scale(Vec3::new(sx * scale, sy * scale, sz * scale));
        parts.push(inst(m, tile, tint, emis));
    };
    let w = [1.0f32, 1.0, 1.0, 0.0];
    match kind {
        P_TREE1 => {
            box_at(&mut parts, 1.3, 0.0, 0.0, 0.5, 2.6, 0.5, textures::T_TRUNK, w, false);
            box_at(&mut parts, 3.2, 0.0, 0.0, 2.8, 2.2, 2.8, textures::T_LEAVES, w, false);
            box_at(&mut parts, 4.7, 0.2, -0.1, 1.8, 1.6, 1.8, textures::T_LEAVES, w, false);
        }
        P_TREE2 => {
            box_at(&mut parts, 1.9, 0.0, 0.0, 0.42, 3.8, 0.42, textures::T_TRUNK, w, false);
            box_at(&mut parts, 4.4, 0.0, 0.0, 2.1, 1.9, 2.1, textures::T_LEAVES, w, false);
            box_at(&mut parts, 5.8, 0.1, 0.1, 1.4, 1.4, 1.4, textures::T_LEAVES, w, false);
        }
        P_ROCK => {
            box_at(&mut parts, 0.45, 0.0, 0.0, 1.4, 0.9, 1.6, textures::T_STONE, w, false);
            box_at(&mut parts, 0.95, 0.25, -0.15, 0.8, 0.5, 0.9, textures::T_STONE, w, false);
        }
        P_MUSH => {
            box_at(&mut parts, 0.45, 0.0, 0.0, 0.24, 0.9, 0.24, textures::T_TRUNK, [0.85, 0.8, 0.7, 0.0], false);
            box_at(&mut parts, 1.05, 0.0, 0.0, 0.95, 0.4, 0.95, textures::T_MUSH, w, true);
        }
        P_HOUSE => {
            box_at(&mut parts, 1.7, 0.0, 0.0, 7.4, 3.4, 5.8, textures::T_HOUSE, w, false);
            box_at(&mut parts, 3.9, 0.0, 0.0, 8.0, 1.4, 6.4, textures::T_ROOF, w, false);
            box_at(&mut parts, 1.0, -1.6, 2.95, 1.3, 2.0, 0.16, textures::T_GATE, [0.8, 0.6, 0.4, 0.0], false);
            box_at(&mut parts, 2.0, -2.6, 2.95, 1.2, 1.2, 0.14, textures::T_WINDOW, w, true);
            box_at(&mut parts, 2.0, 1.9, 2.95, 1.2, 1.2, 0.14, textures::T_WINDOW, w, true);
        }
        P_TOMB => {
            box_at(&mut parts, 0.55, 0.0, 0.0, 0.85, 1.1, 0.3, textures::T_TOMB, w, false);
        }
        P_BRAZIER => {
            box_at(&mut parts, 0.35, 0.0, 0.0, 0.75, 0.7, 0.75, textures::T_STONE, [0.7, 0.7, 0.75, 0.0], false);
            box_at(&mut parts, 0.9, 0.0, 0.0, 0.5, 0.25, 0.5, textures::T_GATE, w, false);
        }
        P_STATUE => {
            box_at(&mut parts, 0.35, 0.0, 0.0, 1.5, 0.7, 1.5, textures::T_STONE, [0.85, 0.9, 0.85, 0.0], false);
            box_at(&mut parts, 1.4, 0.0, 0.0, 1.7, 1.7, 1.7, textures::T_CREEPER, w, false);
        }
        P_CARAVAN => {
            box_at(&mut parts, 1.1, 0.0, 0.0, 5.2, 2.0, 2.5, textures::T_PLANKS, w, false);
            box_at(&mut parts, 2.45, 0.0, 0.0, 5.6, 0.7, 2.9, textures::T_ROOF, w, false);
            for (wx, wz) in [(-1.8, 1.1), (1.8, 1.1), (-1.8, -1.1), (1.8, -1.1)] {
                box_at(&mut parts, 0.42, wx, wz, 0.28, 0.84, 0.84, textures::T_TRUNK, [0.5, 0.5, 0.5, 0.0], false);
            }
        }
        P_PORTAL => {
            box_at(&mut parts, 1.7, -1.3, 0.0, 0.65, 3.4, 0.65, textures::T_GATE, w, true);
            box_at(&mut parts, 1.7, 1.3, 0.0, 0.65, 3.4, 0.65, textures::T_GATE, w, true);
            box_at(&mut parts, 3.6, 0.0, 0.0, 3.3, 0.65, 0.65, textures::T_GATE, w, true);
        }
        _ => {}
    }
    parts
}
