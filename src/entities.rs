// Entités : statistiques (approximations documentées du diff. I), rendu voxel
// des personnages (boîtes instanciées), projectiles, butin, particules.

use crate::atlas::Atlas;
use crate::gfx::InstanceBox;
use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Zombie,
    Skeleton,
    Creeper,
    Necromancer,
}

impl EnemyKind {
    pub fn stats(kind: EnemyKind) -> Stats {
        match kind {
            EnemyKind::Zombie => Stats {
                hp: 40.0,
                dmg: 10.0,
                speed: 2.7,
                souls: 2,
                xp: 5,
                emeralds: (1, 2),
                radius: 0.42,
            },
            EnemyKind::Skeleton => Stats {
                hp: 30.0,
                dmg: 8.0,
                speed: 2.4,
                souls: 2,
                xp: 6,
                emeralds: (1, 2),
                radius: 0.4,
            },
            EnemyKind::Creeper => Stats {
                hp: 30.0,
                dmg: 30.0,
                speed: 3.3,
                souls: 3,
                xp: 8,
                emeralds: (2, 3),
                radius: 0.4,
            },
            EnemyKind::Necromancer => Stats {
                hp: 600.0,
                dmg: 12.0,
                speed: 2.2,
                souls: 20,
                xp: 150,
                emeralds: (20, 30),
                radius: 0.5,
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Stats {
    pub hp: f32,
    pub dmg: f32,
    pub speed: f32,
    pub souls: u32,
    pub xp: u32,
    pub emeralds: (u32, u32),
    pub radius: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EnemyState {
    Idle,
    Chase,
    Windup { t: f32 },
    Attack { t: f32 },
    Cooldown { t: f32 },
    Fuse { t: f32 },
    Summon { t: f32 },
    Cast { t: f32 },
    Teleport { t: f32 },
    Dead { t: f32 },
}

pub struct Enemy {
    pub kind: EnemyKind,
    pub pos: Vec3,
    pub facing: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub state: EnemyState,
    pub walk_phase: f32,
    pub enchanted: bool,
    pub hit_flash: f32,
    pub summon_count: u32,
}

impl Enemy {
    pub fn new(kind: EnemyKind, pos: Vec3, enchanted: bool) -> Self {
        let s = EnemyKind::stats(kind);
        let hp = if enchanted { s.hp * 9.0 } else { s.hp };
        Enemy {
            kind,
            pos,
            facing: 0.0,
            hp,
            max_hp: hp,
            state: EnemyState::Idle,
            walk_phase: 0.0,
            enchanted,
            hit_flash: 0.0,
            summon_count: 0,
        }
    }
    pub fn dmg(&self) -> f32 {
        let base = EnemyKind::stats(self.kind).dmg;
        if self.enchanted {
            base * 2.0
        } else {
            base
        }
    }
    pub fn speed(&self) -> f32 {
        EnemyKind::stats(self.kind).speed * if self.enchanted { 1.15 } else { 1.0 }
    }
    pub fn radius(&self) -> f32 {
        EnemyKind::stats(self.kind).radius * if self.enchanted { 1.15 } else { 1.0 }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ProjectileKind {
    PlayerArrow,
    SkeletonArrow,
    NecroOrb,
    Firework,
}

pub struct Projectile {
    pub kind: ProjectileKind,
    pub pos: Vec3,
    pub vel: Vec3,
    pub dmg: f32,
    pub life: f32,
}

pub struct Pickup {
    pub pos: Vec3,
    pub vel: Vec3,
    pub kind: PickupKind,
    pub age: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PickupKind {
    Emerald,
    Arrows,
    Soul,
    HealthOrb,
}

pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub icon: &'static str,
    pub color: [f32; 4],
    pub kind: u8, // 0 billboard, 1 sol
    pub grav: f32,
}

pub struct DamageNumber {
    pub pos: Vec3,
    pub value: u32,
    pub life: f32,
    pub crit: bool,
    pub color: [f32; 3],
}

/// Ajoute les boîtes d'un personnage à la liste d'instances.
/// `rot` = orientation (0 = face +Z), `walk` = phase de marche, `attack` = 0..1.
#[allow(clippy::too_many_arguments)]
pub fn push_character(
    kind: Character,
    pos: Vec3,
    rot: f32,
    walk_phase: f32,
    attack: f32,
    time: f32,
    hurt: f32,
    fuse: f32,
    out: &mut Vec<InstanceBox>,
    atlas: &Atlas,
) {
    let swing = walk_phase.sin() * 0.6;
    let (tint, extra): ([f32; 3], f32) = if hurt > 0.0 {
        ([2.2, 0.9, 0.9], 0.0)
    } else {
        ([1.0, 1.0, 1.0], 0.0)
    };
    let add = |out: &mut Vec<InstanceBox>,
                   dx: f32,
                   dy: f32,
                   dz: f32,
                   sx: f32,
                   sy: f32,
                   sz: f32,
                   color: [f32; 3],
                   tile: Option<&str>,
                   emissive: f32| {
        let (uvr, uvsel) = match tile {
            Some(t) => {
                let uv = atlas.uv(t);
                ([uv.u0, uv.v0, uv.u1, uv.v1], 0.0)
            }
            None => ([0.0, 0.0, 0.0, 0.0], -1.0),
        };
        // rotation locale des membres : offsets de balancement simples
        out.push(InstanceBox {
            pos_rot: [pos.x + dx, pos.y + dy, pos.z + dz, rot],
            size_uvsel: [sx, sy, sz, uvsel],
            color: [
                color[0] * tint[0],
                color[1] * tint[1],
                color[2] * tint[2],
                1.0,
            ],
            uvr,
            emissive: emissive + extra,
        });
    };
    match kind {
        Character::Hero => {
            // jambes
            let l = swing * 0.9;
            add(out, -0.14, 0.0, l * 0.12, 0.26, 0.55, 0.26, [0.35, 0.3, 0.28], None, 0.0);
            add(out, 0.14, 0.0, -l * 0.12, 0.26, 0.55, 0.26, [0.35, 0.3, 0.28], None, 0.0);
            // torse (tunique turquoise MCD)
            add(out, 0.0, 0.55, 0.0, 0.6, 0.7, 0.34, [0.2, 0.62, 0.66], None, 0.0);
            // bras
            let arm = -swing * 0.9 + attack * -1.6;
            add(out, -0.4, 0.6, arm * 0.14, 0.2, 0.62, 0.2, [0.87, 0.72, 0.58], None, 0.0);
            add(out, 0.4, 0.6, -arm * 0.14, 0.2, 0.62, 0.2, [0.87, 0.72, 0.58], None, 0.0);
            // tête + cheveux
            add(out, 0.0, 1.25, 0.0, 0.52, 0.52, 0.52, [0.9, 0.76, 0.62], Some("villager_face"), 0.0);
            add(out, 0.0, 1.62, -0.02, 0.56, 0.14, 0.56, [0.32, 0.24, 0.18], None, 0.0);
            // épée dans la main droite
            let sword_y = 0.95 + attack * 0.5;
            add(out, 0.52, sword_y, 0.1 + attack * 0.35, 0.1, 0.72, 0.18, [0.88, 0.92, 0.98], None, 0.15);
            add(out, 0.52, sword_y - 0.36, 0.1 + attack * 0.35, 0.3, 0.08, 0.24, [0.45, 0.32, 0.2], None, 0.0);
        }
        Character::Zombie => {
            let l = swing * 0.8;
            add(out, -0.14, 0.0, l * 0.1, 0.26, 0.55, 0.26, [0.25, 0.3, 0.42], None, 0.0);
            add(out, 0.14, 0.0, -l * 0.1, 0.26, 0.55, 0.26, [0.25, 0.3, 0.42], None, 0.0);
            add(out, 0.0, 0.55, 0.0, 0.56, 0.68, 0.32, [0.18, 0.5, 0.45], None, 0.0);
            // bras tendus vers l'avant (zombie classique)
            add(out, -0.38, 1.0, 0.28, 0.18, 0.18, 0.55, [0.31, 0.48, 0.31], None, 0.0);
            add(out, 0.38, 1.0, 0.28, 0.18, 0.18, 0.55, [0.31, 0.48, 0.31], None, 0.0);
            add(out, 0.0, 1.25, 0.0, 0.5, 0.5, 0.5, [0.35, 0.52, 0.35], Some("zombie_face"), 0.0);
        }
        Character::Skeleton => {
            let l = swing * 0.8;
            add(out, -0.13, 0.0, l * 0.1, 0.16, 0.55, 0.16, [0.78, 0.78, 0.72], None, 0.0);
            add(out, 0.13, 0.0, -l * 0.1, 0.16, 0.55, 0.16, [0.78, 0.78, 0.72], None, 0.0);
            add(out, 0.0, 0.55, 0.0, 0.4, 0.6, 0.22, [0.72, 0.72, 0.66], None, 0.0);
            add(out, -0.28, 0.62, 0.12, 0.14, 0.5, 0.14, [0.78, 0.78, 0.72], None, 0.0);
            add(out, 0.28, 0.62, -0.05, 0.14, 0.5, 0.14, [0.78, 0.78, 0.72], None, 0.0);
            add(out, 0.0, 1.18, 0.0, 0.46, 0.46, 0.46, [0.84, 0.84, 0.78], Some("skull_face"), 0.0);
            // arc
            add(out, 0.34, 0.95, 0.3, 0.06, 0.5, 0.06, [0.5, 0.38, 0.24], None, 0.0);
        }
        Character::Creeper => {
            let flash = fuse.clamp(0.0, 1.0);
            let pulse = 1.0 + flash * 0.25 + (time * 12.0 * flash).sin().abs() * 0.05 * flash;
            // 4 pattes
            add(out, -0.15, 0.0, 0.18, 0.22, 0.35, 0.22, [0.3, 0.62, 0.28], None, flash * 0.4);
            add(out, 0.15, 0.0, 0.18, 0.22, 0.35, 0.22, [0.3, 0.62, 0.28], None, flash * 0.4);
            add(out, -0.15, 0.0, -0.18, 0.22, 0.35, 0.22, [0.3, 0.62, 0.28], None, flash * 0.4);
            add(out, 0.15, 0.0, -0.18, 0.22, 0.35, 0.22, [0.3, 0.62, 0.28], None, flash * 0.4);
            // corps
            add(out, 0.0, 0.35, 0.0, 0.56 * pulse, 0.75, 0.34 * pulse, [0.3, 0.64, 0.29], None, flash * 0.5);
            // tête
            add(out, 0.0, 1.1, 0.0, 0.54 * pulse, 0.54 * pulse, 0.54 * pulse, [0.3, 0.64, 0.29], Some("creeper_face"), flash * 0.6);
        }
        Character::Necromancer => {
            let l = swing * 0.5;
            // robe (jupe + torse)
            add(out, 0.0, 0.0, 0.0, 0.75, 0.7, 0.5, [0.28, 0.2, 0.4], None, 0.0);
            add(out, 0.0, 0.7, 0.0, 0.6, 0.75, 0.42, [0.32, 0.23, 0.46], None, 0.0);
            // épaulettes
            add(out, -0.42, 1.38, 0.0, 0.26, 0.18, 0.4, [0.22, 0.16, 0.34], None, 0.0);
            add(out, 0.42, 1.38, 0.0, 0.26, 0.18, 0.4, [0.22, 0.16, 0.34], None, 0.0);
            // crâne + couronne
            add(out, 0.0, 1.42, 0.02, 0.5, 0.5, 0.5, [0.85, 0.85, 0.8], Some("skull_face"), 0.0);
            add(out, 0.0, 1.72, 0.0, 0.56, 0.12, 0.56, [0.85, 0.68, 0.3], None, 0.1);
            add(out, 0.0, 1.84, 0.0, 0.2, 0.16, 0.2, [0.9, 0.74, 0.35], None, 0.1);
            // bâton avec cube d'âme
            add(out, 0.55, 0.35 + l * 0.1, 0.3, 0.09, 1.5, 0.09, [0.3, 0.24, 0.38], None, 0.0);
            add(out, 0.55, 1.95 + l * 0.1, 0.3, 0.26, 0.26, 0.26, [0.55, 0.9, 1.0], None, 0.9);
        }
        Character::Villager => {
            add(out, 0.0, 0.0, 0.0, 0.5, 0.85, 0.36, [0.6, 0.45, 0.3], None, 0.0);
            add(out, 0.0, 0.85, 0.05, 0.46, 0.5, 0.3, [0.72, 0.55, 0.38], None, 0.0);
            add(out, 0.0, 1.2, 0.0, 0.46, 0.46, 0.46, [0.87, 0.7, 0.55], Some("villager_face"), 0.0);
            add(out, 0.0, 1.48, -0.02, 0.4, 0.1, 0.4, [0.3, 0.24, 0.18], None, 0.0);
        }
    }
}

#[derive(Clone, Copy)]
pub enum Character {
    Hero,
    Zombie,
    Skeleton,
    Creeper,
    Necromancer,
    Villager,
}

/// Cube du projectile (flèche / orbe / feu d'artifice).
pub fn push_projectile(p: &Projectile, out: &mut Vec<InstanceBox>, time: f32) {
    match p.kind {
        ProjectileKind::PlayerArrow | ProjectileKind::SkeletonArrow => {
            let dir = p.vel.normalize_or_zero();
            let yaw = dir.z.atan2(dir.x);
            out.push(InstanceBox {
                pos_rot: [p.pos.x, p.pos.y - 0.06, p.pos.z, yaw],
                size_uvsel: [0.55, 0.05, 0.05, -1.0],
                color: [0.75, 0.68, 0.55, 1.0],
                uvr: [0.0; 4],
                emissive: 0.0,
            });
            out.push(InstanceBox {
                pos_rot: [p.pos.x, p.pos.y - 0.06, p.pos.z, yaw],
                size_uvsel: [0.12, 0.12, 0.12, -1.0],
                color: [0.85, 0.85, 0.85, 1.0],
                uvr: [0.0; 4],
                emissive: 0.0,
            });
        }
        ProjectileKind::NecroOrb => {
            out.push(InstanceBox {
                pos_rot: [p.pos.x, p.pos.y, p.pos.z, time * 6.0],
                size_uvsel: [0.34, 0.34, 0.34, -1.0],
                color: [0.65, 0.35, 1.0, 1.0],
                uvr: [0.0; 4],
                emissive: 0.85,
            });
        }
        ProjectileKind::Firework => {
            out.push(InstanceBox {
                pos_rot: [p.pos.x, p.pos.y, p.pos.z, 0.0],
                size_uvsel: [0.16, 0.3, 0.16, -1.0],
                color: [0.95, 0.85, 0.5, 1.0],
                uvr: [0.0; 4],
                emissive: 0.5,
            });
        }
    }
}
