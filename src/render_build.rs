/// Construit les listes de rendu de la frame à partir du monde : culling frustum
/// des chunks et des props, instanciation des rigs (héros, monstres, props
/// interactifs), billboards triés, et sélection des 16 lumières les plus proches.
use glam::{Mat4, Vec2, Vec3, Vec4};
use std::f32::consts::TAU;

use crate::game::enemy as em;
use crate::game::player as pl;
use crate::game::{fx, World, PR_ARROW, PR_BONE, PR_ORB, PR_ROCKET, PR_ZAP};
use crate::gfx::camera::Camera;
use crate::gfx::textures as tex;
use crate::gfx::{BillRaw, FrameData, InstRaw};
use crate::level::{self, LightSrc};

/// Extraction des 6 plans du frustum (méthode de Gribb–Hartmann).
pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    pub fn from(m: Mat4) -> Self {
        let r = [
            m.row(0),
            m.row(1),
            m.row(2),
            m.row(3),
        ];
        let mk = |a: usize, sign: f32| {
            let p = r[3] + sign * r[a];
            let n = Vec3::new(p.x, p.y, p.z);
            let len = n.length().max(0.0001);
            Vec4::new(n.x / len, n.y / len, n.z / len, p.w / len)
        };
        Frustum {
            planes: [
                mk(0, 1.0),
                mk(0, -1.0),
                mk(1, 1.0),
                mk(1, -1.0),
                mk(2, 1.0),
                mk(2, -1.0),
            ],
        }
    }

    pub fn sphere(&self, c: Vec3, r: f32) -> bool {
        self.planes
            .iter()
            .all(|p| p.truncate().dot(c) + p.w > -r)
    }
}

fn inst(m: Mat4, tile: usize, tint: [f32; 4], emissive: bool) -> InstRaw {
    let u0 = (tile % tex::GRID) as f32 / tex::GRID as f32;
    let v0 = (tile / tex::GRID) as f32 / tex::GRID as f32;
    let s = 1.0 / tex::GRID as f32;
    let inset = s * 0.02;
    InstRaw {
        model: m.to_cols_array_2d(),
        tint: [tint[0], tint[1], tint[2], f32::from(emissive)],
        uv0: [u0 + inset, v0 + inset],
        uvs: [s - 2.0 * inset, s - 2.0 * inset],
    }
}

/// Empile un cube orienté (taille avant rotation, pivot au centre).
#[allow(clippy::too_many_arguments)] // helpers de dessin à paramètres alignés
fn cube(out: &mut Vec<InstRaw>, m: Mat4, sx: f32, sy: f32, sz: f32, tile: usize, tint: [f32; 4], emis: bool) {
    out.push(inst(m * Mat4::from_scale(Vec3::new(sx, sy, sz)), tile, tint, emis));
}

/// Membre articulé : pivot + rotation X + cube pendant vers le bas.
#[allow(clippy::too_many_arguments)] // helpers de dessin à paramètres alignés
fn limb(out: &mut Vec<InstRaw>, base: Mat4, pivot: Vec3, angle: f32, len: f32, th: f32, tile: usize, tint: [f32; 4]) {
    let m = base
        * Mat4::from_translation(pivot)
        * Mat4::from_rotation_x(angle)
        * Mat4::from_translation(Vec3::new(0.0, -len * 0.5, 0.0))
        * Mat4::from_scale(Vec3::new(th, len, th));
    out.push(inst(m, tile, tint, false));
}

fn at(x: f32, y: f32, z: f32, yaw: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(x, y, z)) * Mat4::from_rotation_y(yaw)
}

fn model_inst(m: Mat4) -> InstRaw {
    InstRaw {
        model: m.to_cols_array_2d(),
        tint: [1.0, 1.0, 1.0, 0.0],
        uv0: [0.0, 0.0],
        uvs: [1.0, 1.0],
    }
}

/// Rig humanoïde commun (héros, zombie, villageois, vindicateur…).
#[allow(clippy::too_many_arguments)] // helpers de dessin à paramètres alignés
fn humanoid(
    out: &mut Vec<InstRaw>,
    pos: Vec2,
    yaw: f32,
    walk: f32,
    swing: f32,
    tile: usize,
    tint: [f32; 4],
    scale: f32,
    weapon: u8,
    bow_draw: f32,
) {
    let base = at(pos.x, 0.0, pos.y, yaw) * Mat4::from_scale(Vec3::splat(scale));
    let sw = walk.sin() * 0.65;
    limb(out, base, Vec3::new(-0.13, 0.78, 0.0), sw, 0.78, 0.22, tile, tint);
    limb(out, base, Vec3::new(0.13, 0.78, 0.0), -sw, 0.78, 0.22, tile, tint);
    cube(out, base * Mat4::from_translation(Vec3::new(0.0, 1.12, 0.0)), 0.52, 0.68, 0.3, tile, tint, false);
    // Bras : le droit porte l'animation de frappe, le gauche peut bander l'arc.
    let ra = sw * 0.5 - swing * 2.1;
    let la = -sw * 0.5 - bow_draw * 1.3;
    limb(out, base, Vec3::new(-0.34, 1.42, 0.0), la, 0.62, 0.18, tile, tint);
    limb(out, base, Vec3::new(0.34, 1.42, 0.0), ra, 0.62, 0.18, tile, tint);
    // Tête avec le visage.
    cube(out, base * Mat4::from_translation(Vec3::new(0.0, 1.72, 0.0)), 0.44, 0.44, 0.44, tile, tint, false);
    let white = [1.0, 1.0, 1.0, 0.0];
    match weapon {
        1 => {
            // Épée dans la main droite.
            let m = base
                * Mat4::from_translation(Vec3::new(0.34, 1.42, 0.0))
                * Mat4::from_rotation_x(ra)
                * Mat4::from_translation(Vec3::new(0.0, -0.62, 0.0))
                * Mat4::from_rotation_x(1.57);
            cube(out, m, 0.09, 0.7, 0.16, tex::T_WHITE, [0.85, 0.88, 0.95, 0.0], false);
        }
        2 => {
            // Hache du vindicateur.
            let m = base
                * Mat4::from_translation(Vec3::new(0.34, 1.42, 0.0))
                * Mat4::from_rotation_x(ra)
                * Mat4::from_translation(Vec3::new(0.0, -0.55, 0.0));
            cube(out, m, 0.08, 0.8, 0.08, tex::T_TRUNK, white, false);
            cube(
                out,
                m * Mat4::from_translation(Vec3::new(0.0, 0.32, 0.08)),
                0.1,
                0.3,
                0.36,
                tex::T_WHITE,
                [0.8, 0.82, 0.88, 0.0],
                false,
            );
        }
        3 => {
            // Arc du squelette / du héros.
            let m = base
                * Mat4::from_translation(Vec3::new(-0.34, 1.42, 0.0))
                * Mat4::from_rotation_x(la)
                * Mat4::from_translation(Vec3::new(0.0, -0.45, 0.05))
                * Mat4::from_rotation_z(1.57);
            cube(out, m, 0.06, 0.85, 0.06, tex::T_TRUNK, white, false);
            if bow_draw > 0.0 {
                cube(
                    out,
                    m * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    0.05,
                    0.7,
                    0.05,
                    tex::T_WHITE,
                    [0.9, 0.9, 0.9, 0.0],
                    false,
                );
            }
        }
        _ => {}
    }
}

/// Rig flottant (enchanteur, Nécromancien) : robe + tête + léger flottement.
#[allow(clippy::too_many_arguments)] // helpers de dessin à paramètres alignés
fn robed(
    out: &mut Vec<InstRaw>,
    pos: Vec2,
    yaw: f32,
    t: f32,
    tile: usize,
    tint: [f32; 4],
    scale: f32,
    staff: bool,
) {
    let bob = (t * 2.2).sin() * 0.09;
    let base = at(pos.x, 0.25 + bob, pos.y, yaw) * Mat4::from_scale(Vec3::splat(scale));
    cube(out, base * Mat4::from_translation(Vec3::new(0.0, 0.6, 0.0)), 0.6, 1.2, 0.42, tile, tint, false);
    cube(out, base * Mat4::from_translation(Vec3::new(0.0, 1.42, 0.0)), 0.46, 0.46, 0.46, tile, tint, false);
    cube(out, base * Mat4::from_translation(Vec3::new(0.0, 1.62, 0.06)), 0.4, 0.14, 0.3, tile, tint, false);
    if staff {
        let m = base * Mat4::from_translation(Vec3::new(0.42, 1.0, 0.1));
        cube(out, m, 0.09, 2.0, 0.09, tex::T_TRUNK, tint, false);
        cube(
            out,
            m * Mat4::from_translation(Vec3::new(0.0, 1.1, 0.0)),
            0.24,
            0.24,
            0.24,
            tex::T_ORB,
            tint,
            true,
        );
    }
}

fn enemy_rig(out: &mut Vec<InstRaw>, e: &em::Enemy) {
    let tint = em::tint(e);
    let t = [tint.x, tint.y, tint.z, tint.w];
    match e.kind {
        level::K_ZOMBIE => humanoid(out, e.pos, e.yaw, e.walk, 0.0, tex::T_ZOMBIE, t, 1.0, 0, 0.0),
        level::K_SKELETON => humanoid(out, e.pos, e.yaw, e.walk, 0.0, tex::T_SKELETON, t, 0.95, 3, 0.0),
        level::K_SPIDER => {
            let base = at(e.pos.x, 0.0, e.pos.y, e.yaw);
            let bob = (e.walk * 2.0).sin() * 0.05;
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, 0.5 + bob, -0.1)), 0.95, 0.45, 1.2, tex::T_SPIDER, t, false);
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, 0.55 + bob, 0.62)), 0.5, 0.4, 0.45, tex::T_SPIDER, t, false);
            for side in [-1.0f32, 1.0] {
                for k in 0..4u8 {
                    let a = e.walk * 1.6 + k as f32 * 1.57;
                    let m = base
                        * Mat4::from_translation(Vec3::new(side * 0.55, 0.45 + bob, -0.5 + k as f32 * 0.32))
                        * Mat4::from_rotation_y(side * (0.5 + (a.sin()) * 0.18));
                    cube(out, m, 0.75, 0.09, 0.1, tex::T_SPIDER, t, false);
                }
            }
        }
        level::K_CREEPER => {
            let base = at(e.pos.x, 0.0, e.pos.y, e.yaw);
            let mut tint = t;
            if e.state == 5 && (e.t * 8.0) as i32 % 2 == 0 {
                tint = [2.4, 2.4, 2.4, 0.0];
            }
            let sw = e.walk.sin() * 0.5;
            for (lx, lz, a) in [(-0.16, 0.2, sw), (0.16, 0.2, -sw), (-0.16, -0.2, -sw), (0.16, -0.2, sw)] {
                limb(out, base, Vec3::new(lx, 0.35, lz), a, 0.35, 0.2, tex::T_CREEPER, tint);
            }
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, 0.9, 0.0)), 0.52, 1.1, 0.34, tex::T_CREEPER, tint, false);
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, 1.66, 0.0)), 0.5, 0.5, 0.5, tex::T_CREEPER, tint, false);
        }
        level::K_VINDICATOR => humanoid(out, e.pos, e.yaw, e.walk, if e.state == 2 { 1.0 } else { 0.0 }, tex::T_VINDICATOR, t, 1.18, 2, 0.0),
        level::K_ENCHANTER => robed(out, e.pos, e.yaw, e.walk, tex::T_ENCHANTER, t, 1.0, false),
        _ => robed(out, e.pos, e.yaw, e.walk, tex::T_NECRO, t, 1.32, true),
    }
}

fn player_rig(out: &mut Vec<InstRaw>, p: &pl::Player) {
    let tint: [f32; 4] = if p.hurt_flash > 0.0 {
        [2.4, 1.1, 1.1, 0.0]
    } else {
        [1.0, 1.0, 1.0, 0.0]
    };
    let swing = if p.swing_t > 0.0 { (p.swing_t * 1.35).min(1.0) } else { 0.0 };
    if p.state == 1 {
        // Roulade : corps replié qui tourne.
        let a = (p.t_state / 0.5) * TAU;
        let base = at(p.pos.x, 0.0, p.pos.y, p.yaw)
            * Mat4::from_translation(Vec3::new(0.0, 0.55, 0.0))
            * Mat4::from_rotation_x(-a)
            * Mat4::from_translation(Vec3::new(0.0, -0.55, 0.0));
        cube(out, base * Mat4::from_translation(Vec3::new(0.0, 0.9, 0.0)), 0.7, 0.8, 0.6, tex::T_HERO, tint, false);
        return;
    }
    let bow = if p.bow_charge > 0.0 { p.bow_charge } else { 0.0 };
    humanoid(out, p.pos, p.yaw, p.walk, swing, tex::T_HERO, tint, 1.05, if bow > 0.0 { 3 } else { 1 }, bow);
}

#[allow(clippy::too_many_arguments)] // helpers de dessin à paramètres alignés
fn inter_rig(out: &mut Vec<InstRaw>, inter: &crate::game::Inter, time: f32) {
    let white = [1.0, 1.0, 1.0, 0.0];
    let base = at(inter.x, 0.0, inter.z, 0.0);
    match inter.kind {
        level::I_VILLAGER | level::I_MERCHANT => {
            if !inter.used {
                // Cage : poteaux + barreaux + porte.
                for (dx, dz) in [(-0.55, -0.55), (0.55, -0.55), (-0.55, 0.55), (0.55, 0.55)] {
                    cube(out, base * Mat4::from_translation(Vec3::new(dx, 1.0, dz)), 0.14, 2.0, 0.14, tex::T_CAGE, white, false);
                }
                for k in -1..=1 {
                    cube(out, base * Mat4::from_translation(Vec3::new(k as f32 * 0.27, 1.0, 0.55)), 0.09, 2.0, 0.09, tex::T_CAGE, white, false);
                    cube(out, base * Mat4::from_translation(Vec3::new(k as f32 * 0.27, 1.0, -0.55)), 0.09, 2.0, 0.09, tex::T_CAGE, white, false);
                }
                cube(out, base * Mat4::from_translation(Vec3::new(0.0, 2.05, 0.0)), 1.3, 0.14, 1.3, tex::T_CAGE, white, false);
                // Porte qui glisse vers le haut à l'ouverture.
                let dy = 1.0 + inter.anim.min(1.0) * 2.2;
                cube(out, base * Mat4::from_translation(Vec3::new(0.0, dy, 0.0)), 1.24, 1.9, 0.5, tex::T_CAGE, white, false);
                let tile = if inter.kind == level::I_MERCHANT { tex::T_MERCHANT } else { tex::T_VILLAGER };
                humanoid(out, Vec2::new(inter.x, inter.z), (time * 0.8).sin() * 0.6, 0.0, 0.0, tile, white, 0.95, 0, 0.0);
            } else if inter.anim > 0.0 {
                let tile = if inter.kind == level::I_MERCHANT { tex::T_MERCHANT } else { tex::T_VILLAGER };
                humanoid(out, Vec2::new(inter.x + 1.4, inter.z + 0.8), 2.2, 0.0, 0.0, tile, white, 0.95, 0, 0.0);
            }
        }
        level::I_CHEST => {
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, 0.35, 0.0)), 1.15, 0.7, 0.85, tex::T_PLANKS, white, false);
            let lid = base
                * Mat4::from_translation(Vec3::new(0.0, 0.72, -0.35))
                * Mat4::from_rotation_x(-inter.anim.min(1.0) * 1.3)
                * Mat4::from_translation(Vec3::new(0.0, 0.12, 0.35));
            cube(out, lid, 1.15, 0.26, 0.85, tex::T_PLANKS, white, false);
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, 0.5, 0.44)), 0.18, 0.3, 0.08, tex::T_EMERALD, white, false);
        }
        level::I_BUTTON => {
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0)), 0.9, 1.0, 0.9, tex::T_STONE, white, false);
            let press = if inter.used { 0.06 } else { 0.16 };
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, 1.05, 0.0)), 0.34, press, 0.34, tex::T_PLATE, white, false);
        }
        level::I_PLATE => {
            let press = if inter.used { 0.05 } else { 0.16 };
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, press, 0.0)), 1.2, 0.12, 1.2, tex::T_PLATE, white, false);
        }
        level::I_PARCH => {
            let bob = (time * 2.0).sin() * 0.12;
            cube(out, base * Mat4::from_translation(Vec3::new(0.0, 0.9 + bob, 0.0)), 0.5, 0.6, 0.08, tex::T_PARCH, white, true);
        }
        level::I_RUNEGATE => {
            let rise = inter.anim.min(1.0) * 3.0;
            for off in [-2.4f32, 0.0, 2.4] {
                let m = if inter.data == 0 {
                    base * Mat4::from_translation(Vec3::new(0.0, 1.4 - rise, off))
                } else {
                    base * Mat4::from_translation(Vec3::new(off, 1.4 - rise, 0.0))
                };
                let (sx, sz) = if inter.data == 0 { (0.6, 1.9) } else { (1.9, 0.6) };
                cube(out, m, sx, 2.8, sz, tex::T_GATE, white, false);
            }
        }
        _ => {}
    }
}

pub fn build(w: &World, cam: &Camera) -> FrameData {
    let vp = cam.view_proj();
    let fr = Frustum::from(vp);
    let mut fd = FrameData {
        cam: *cam,
        chunks: Vec::new(),
        insts: Vec::new(),
        model_insts: Vec::new(),
        model_draws: Vec::new(),
        bills: Vec::new(),
        bills_add: Vec::new(),
        lights_pos: [[0.0; 4]; 16],
        lights_col: [[0.0; 4]; 16],
        time: [w.time, w.hurt_flash, 0.0, 0.0],
    };

    // --- Chunks visibles (tri près → loin) -------------------------------------
    let mut vis: Vec<(u32, u32, f32)> = w
        .level
        .chunk_draws
        .iter()
        .filter(|(_, _, c, r)| fr.sphere(*c, *r))
        .map(|(f, n, c, _)| (*f, *n, (c - cam.eye).length_squared()))
        .collect();
    vis.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    fd.chunks = vis.iter().map(|(f, n, _)| (*f, *n)).collect();

    // --- Props des chunks visibles ----------------------------------------------
    let mut seen = vec![false; w.level.props.len()];
    let mut model_insts: Vec<(u32, InstRaw)> = Vec::new();
    for &(_f, _n, c, r) in &w.level.chunk_draws {
        if !fr.sphere(c, r) {
            continue;
        }
        let ci = ((c.z / level::TILE) as usize / level::CHUNK) * level::CHUNKS
            + ((c.x / level::TILE) as usize / level::CHUNK);
        for &pi in &w.level.prop_grid[ci] {
            if seen[pi as usize] {
                continue;
            }
            seen[pi as usize] = true;
            let p = &w.level.props[pi as usize];
            if !fr.sphere(Vec3::new(p.x, 1.5, p.z), 3.5) {
                continue;
            }
            if let Some(mid) = p.model {
                model_insts.push((mid, model_inst(at(p.x, 0.0, p.z, p.yaw))));
            } else {
                fd.insts.extend_from_slice(&p.parts);
            }
        }
    }
    model_insts.sort_by_key(|(mid, _)| *mid);
    let mut model_draws: Vec<(u32, u32, u32)> = Vec::new();
    for (idx, (mid, raw)) in model_insts.into_iter().enumerate() {
        let idx = idx as u32;
        match model_draws.last_mut() {
            Some((m, _, count)) if *m == mid => *count += 1,
            _ => model_draws.push((mid, idx, 1)),
        }
        fd.model_insts.push(raw);
    }
    fd.model_draws = model_draws;

    // --- Billboards statiques -----------------------------------------------------
    for (pos, b) in &w.level.static_bills {
        if fr.sphere(*pos, b.pos_size[3] + 0.5) {
            if b.params[1] == tex::T_WEB as f32 {
                fd.bills.push(*b);
            } else {
                fd.bills_add.push(*b);
            }
        }
    }

    // --- Entités ---------------------------------------------------------------------
    player_rig(&mut fd.insts, &w.player);
    for e in &w.enemies {
        if fr.sphere(Vec3::new(e.pos.x, 1.0, e.pos.y), 2.0) {
            enemy_rig(&mut fd.insts, e);
        }
    }
    for inter in &w.inters {
        if fr.sphere(Vec3::new(inter.x, 1.0, inter.z), 3.0) {
            inter_rig(&mut fd.insts, inter, w.time);
        }
    }

    // --- Ramassables ---------------------------------------------------------------------
    for pk in &w.pickups {
        let bob = (pk.t * 3.0).sin() * 0.1;
        let (tile, tint) = match pk.kind {
            fx::PK_EMERALD => (tex::T_EMERALD, [0.6, 1.0, 0.75, 0.0]),
            fx::PK_ARROWS => (tex::T_ARROW, [1.0, 1.0, 1.0, 0.0]),
            fx::PK_APPLE => (tex::T_APPLE, [1.0, 1.0, 1.0, 0.0]),
            _ => (tex::T_WHITE, [0.4, 1.0, 0.5, 1.0]),
        };
        if pk.kind == fx::PK_SOUL {
            fd.bills_add.push(BillRaw {
                pos_size: [pk.pos.x, 0.7 + bob, pk.pos.y, 0.8],
                color: [0.4, 1.0, 0.5, 0.85],
                params: [0.0, tex::T_SOUL as f32, 0.0, 0.0],
            });
        } else {
            let m = at(pk.pos.x, 0.5 + bob, pk.pos.y, pk.t * 1.7)
                * Mat4::from_scale(Vec3::splat(if pk.kind == fx::PK_ARROWS { 0.7 } else { 0.45 }));
            fd.insts.push(inst(m, tile, tint, false));
            fd.bills_add.push(BillRaw {
                pos_size: [pk.pos.x, 0.6 + bob, pk.pos.y, 0.55],
                color: [1.0, 1.0, 1.0, 0.3],
                params: [0.0, tex::T_GLOW as f32, 0.0, 0.0],
            });
        }
    }

    // --- Projectiles ---------------------------------------------------------------------
    for pr in &w.projectiles {
        match pr.kind {
            PR_ARROW | PR_BONE => {
                let flat = Vec2::new(pr.vel.x, pr.vel.z).length();
                let yaw = pr.vel.z.atan2(pr.vel.x);
                let pitch = (pr.vel.y / flat.max(0.001)).atan();
                let m = at(pr.pos.x, pr.pos.y, pr.pos.z, yaw)
                    * Mat4::from_rotation_z(-pitch)
                    * Mat4::from_scale(Vec3::new(0.09, 0.09, 0.75));
                fd.insts.push(inst(m, tex::T_ARROW, [1.0, 1.0, 1.0, 0.0], false));
            }
            PR_ZAP => {
                fd.bills_add.push(BillRaw {
                    pos_size: [pr.pos.x, pr.pos.y, pr.pos.z, 0.5],
                    color: [0.8, 0.4, 1.0, 0.9],
                    params: [0.0, tex::T_ORB as f32, 0.0, 0.0],
                });
            }
            PR_ORB => {
                fd.bills_add.push(BillRaw {
                    pos_size: [pr.pos.x, pr.pos.y, pr.pos.z, 1.0],
                    color: [1.0, 0.5, 1.0, 1.0],
                    params: [0.0, tex::T_ORB as f32, 0.0, 0.0],
                });
                fd.bills_add.push(BillRaw {
                    pos_size: [pr.pos.x, pr.pos.y, pr.pos.z, 1.8],
                    color: [0.6, 0.2, 0.9, 0.4],
                    params: [0.0, tex::T_GLOW as f32, 0.0, 0.0],
                });
            }
            PR_ROCKET => {
                let yaw = pr.vel.z.atan2(pr.vel.x);
                let m = at(pr.pos.x, pr.pos.y, pr.pos.z, yaw) * Mat4::from_scale(Vec3::new(0.2, 0.2, 0.5));
                fd.insts.push(inst(m, tex::T_FIREWORK, [1.0, 1.0, 1.0, 0.0], false));
            }
            _ => {}
        }
    }

    // --- Particules (tri arrière → avant) ------------------------------------------------
    let mut bills: Vec<(f32, BillRaw)> = Vec::new();
    let mut adds: Vec<(f32, BillRaw)> = Vec::new();
    for p in &w.particles {
        let a = (p.ttl / p.ttl0).clamp(0.0, 1.0);
        let b = BillRaw {
            pos_size: [p.pos.x, p.pos.y, p.pos.z, p.size * (0.4 + 0.6 * a)],
            color: [p.color[0], p.color[1], p.color[2], p.color[3] * a],
            params: [f32::from(p.ground), p.tile as f32, 0.0, 0.0],
        };
        let d = (Vec3::new(p.pos.x, p.pos.y, p.pos.z) - cam.eye).length_squared();
        if p.add {
            adds.push((d, b));
        } else {
            bills.push((d, b));
        }
    }
    bills.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    adds.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    fd.bills.extend(bills.into_iter().map(|(_, b)| b));
    fd.bills_add.extend(adds.into_iter().map(|(_, b)| b));

    // --- Lumières : slot 0 = héros, puis les 15 sources statiques les plus proches ---
    let p = w.player.pos;
    fd.lights_pos[0] = [p.x, 1.5, p.y, 7.0];
    fd.lights_col[0] = [1.0, 0.82, 0.55, 0.95];
    let mut lights: Vec<(f32, &LightSrc)> = w
        .level
        .lights
        .iter()
        .map(|l| ((l.pos - cam.eye).length_squared(), l))
        .collect();
    lights.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (i, (_, l)) in lights.iter().take(15).enumerate() {
        fd.lights_pos[i + 1] = [l.pos.x, l.pos.y, l.pos.z, l.radius];
        fd.lights_col[i + 1] = [l.color[0], l.color[1], l.color[2], l.intensity];
    }

    fd
}
