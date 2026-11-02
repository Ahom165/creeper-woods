/// Effets de jeu : particules (billboards), textes de dégâts flottants, messages
/// toast, et objets ramassables (émeraudes, flèches, pommes, orbes d'âme).
use glam::{Vec2, Vec3};

#[derive(Clone, Copy)]
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub ttl: f32,
    pub ttl0: f32,
    pub size: f32,
    pub color: [f32; 4],
    pub tile: usize,
    pub ground: bool,
    pub add: bool,
    pub grav: f32,
}

impl Particle {
    pub fn alive(&self) -> bool {
        self.ttl > 0.0
    }
}

#[derive(Clone)]
pub struct FloatText {
    pub pos: Vec3,
    pub text: String,
    pub color: [f32; 4],
    pub ttl: f32,
    pub size: f32,
}

#[derive(Clone)]
pub struct Toast {
    pub text: String,
    pub ttl: f32,
}

pub const PK_EMERALD: u8 = 0;
pub const PK_ARROWS: u8 = 1;
pub const PK_APPLE: u8 = 2;
pub const PK_SOUL: u8 = 3;

pub struct Pickup {
    pub kind: u8,
    pub pos: Vec2,
    pub vel: Vec2,
    pub ttl: f32,
    /// Valeur : émeraudes, flèches, PV rendus, âmes.
    pub amount: f32,
    pub t: f32,
}

pub const P_SOFT: usize = crate::gfx::textures::T_SOFT;
pub const P_GLOW: usize = crate::gfx::textures::T_GLOW;
pub const P_FIRE: usize = crate::gfx::textures::T_FIRE;

/// Étincelles blanches à l'impact.
pub fn spark(particles: &mut Vec<Particle>, pos: Vec3, n: usize) {
    let mut seed = (pos.x * 31.7 + pos.z * 17.3) as u64 | 1;
    for _ in 0..n {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let fx = ((seed >> 33) & 0xFFFF) as f32 / 65535.0 - 0.5;
        let fz = ((seed >> 17) & 0xFFFF) as f32 / 65535.0 - 0.5;
        let fy = ((seed & 0xFF) as f32 / 255.0) * 2.5;
        particles.push(Particle {
            pos,
            vel: Vec3::new(fx * 4.0, fy + 1.5, fz * 4.0),
            ttl: 0.35,
            ttl0: 0.35,
            size: 0.22,
            color: [1.0, 0.95, 0.8, 0.95],
            tile: P_GLOW,
            ground: false,
            add: true,
            grav: 9.0,
        });
    }
}

/// Bouffée grise (mort, poussière).
pub fn poof(particles: &mut Vec<Particle>, pos: Vec3, n: usize) {
    for i in 0..n {
        let a = (i as f32) * 0.7;
        particles.push(Particle {
            pos,
            vel: Vec3::new(a.cos() * 1.6, 1.2, a.sin() * 1.6),
            ttl: 0.6,
            ttl0: 0.6,
            size: 0.7,
            color: [0.85, 0.85, 0.85, 0.55],
            tile: P_SOFT,
            ground: false,
            add: false,
            grav: -0.5,
        });
    }
}

/// Explosion (creeper, roquette d'artifice).
pub fn explosion(particles: &mut Vec<Particle>, pos: Vec3) {
    for i in 0..18 {
        let a = (i as f32) * 0.35;
        let r = 2.0 + (i % 5) as f32 * 0.55;
        particles.push(Particle {
            pos,
            vel: Vec3::new(a.cos() * r * 1.6, 2.4, a.sin() * r * 1.6),
            ttl: 0.5,
            ttl0: 0.5,
            size: 1.1,
            color: [1.0, 0.7, 0.25, 0.95],
            tile: P_FIRE,
            ground: false,
            add: true,
            grav: 2.0,
        });
    }
    for i in 0..10 {
        let a = (i as f32) * 0.63;
        particles.push(Particle {
            pos,
            vel: Vec3::new(a.cos() * 2.2, 1.0, a.sin() * 2.2),
            ttl: 0.8,
            ttl0: 0.8,
            size: 1.4,
            color: [0.6, 0.6, 0.6, 0.5],
            tile: P_SOFT,
            ground: false,
            add: false,
            grav: -0.4,
        });
    }
}

/// Bouffée d'âmes vertes (mort d'un ennemi enchanté, guérison d'âme).
pub fn soul_burst(particles: &mut Vec<Particle>, pos: Vec3, n: usize) {
    for i in 0..n {
        let a = (i as f32) * 0.9;
        particles.push(Particle {
            pos,
            vel: Vec3::new(a.cos() * 0.7, 2.2, a.sin() * 0.7),
            ttl: 0.9,
            ttl0: 0.9,
            size: 0.45,
            color: [0.35, 1.0, 0.45, 0.85],
            tile: P_GLOW,
            ground: false,
            add: true,
            grav: -1.0,
        });
    }
}

/// Aura violette montante (enchanter, ennemis enchantés).
pub fn aura(particles: &mut Vec<Particle>, pos: Vec3) {
    particles.push(Particle {
        pos: pos + Vec3::new(0.0, 0.3, 0.0),
        vel: Vec3::new(0.0, 1.4, 0.0),
        ttl: 0.7,
        ttl0: 0.7,
        size: 0.5,
        color: [0.7, 0.35, 1.0, 0.6],
        tile: P_GLOW,
        ground: false,
        add: true,
        grav: 0.0,
    });
}

/// Souffle du cor du vent.
pub fn wind_cone(particles: &mut Vec<Particle>, pos: Vec3, dir: Vec2) {
    for i in 0..14 {
        let t = (i as f32) / 14.0;
        let spread = (i as f32 * 0.9).sin() * 0.8;
        let d = dir.normalize_or_zero();
        let perp = Vec2::new(-d.y, d.x);
        let p = pos + Vec3::new(d.x, 0.9, d.y).normalize_or_zero() * (1.0 + t * 4.5)
            + Vec3::new(perp.x, 0.0, perp.y) * spread;
        particles.push(Particle {
            pos: p,
            vel: Vec3::new(d.x, 0.0, d.y) * 8.0,
            ttl: 0.3,
            ttl0: 0.3,
            size: 0.6 + t,
            color: [0.8, 0.95, 1.0, 0.5],
            tile: P_SOFT,
            ground: false,
            add: false,
            grav: 0.0,
        });
    }
}

/// Traînée de fusée d'artifice.
pub fn rocket_trail(particles: &mut Vec<Particle>, pos: Vec3) {
    particles.push(Particle {
        pos,
        vel: Vec3::new(0.0, 0.4, 0.0),
        ttl: 0.45,
        ttl0: 0.45,
        size: 0.42,
        color: [1.0, 0.75, 0.35, 0.85],
        tile: P_FIRE,
        ground: false,
        add: true,
        grav: 0.0,
    });
}

/// Cercle d'invocation du Nécromancien.
pub fn summon_ring(particles: &mut Vec<Particle>, center: Vec3, radius: f32) {
    for i in 0..20 {
        let a = (i as f32) * 0.314;
        particles.push(Particle {
            pos: center + Vec3::new(a.cos() * radius, 0.2, a.sin() * radius),
            vel: Vec3::new(0.0, 2.0, 0.0),
            ttl: 1.2,
            ttl0: 1.2,
            size: 0.6,
            color: [0.6, 0.3, 1.0, 0.7],
            tile: P_GLOW,
            ground: true,
            add: true,
            grav: 0.0,
        });
    }
}

/// Ramassable qui apparaît avec un petit saut.
pub fn drop_pickup(pickups: &mut Vec<Pickup>, kind: u8, pos: Vec2, amount: f32, rng: &mut crate::rng::Rng) {
    pickups.push(Pickup {
        kind,
        pos,
        vel: Vec2::new(rng.range_f32(-2.5, 2.5), rng.range_f32(-2.5, 2.5)),
        ttl: 30.0,
        amount,
        t: 0.0,
    });
}
