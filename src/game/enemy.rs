/// Ennemis de Creeper Woods et IA associée. Équilibres issus du bestiaire source :
/// zombie, squelette (archer), creeper (mèche), araignée (rapide), vindicateur
/// (hache), enchanteur (aura violette → ennemis enchantés) et le Nécromancien
/// (boss, 600 PV, invocations plafonnées, orbes repoussantes, téléportations).
use glam::{Vec2, Vec3, Vec4};
use std::f32::consts::TAU;

use super::fx;
use super::{dist2, World, PR_BONE, PR_ORB, PR_ZAP};
use crate::level;

pub struct Def {
    pub hp: f32,
    pub speed: f32,
    pub dmg: f32,
    pub reach: f32,
    pub cd: f32,
    pub windup: f32,
    pub radius: f32,
}

pub fn def(kind: u8) -> Def {
    match kind {
        level::K_ZOMBIE => Def { hp: 30.0, speed: 2.7, dmg: 10.0, reach: 1.8, cd: 1.1, windup: 0.45, radius: 0.45 },
        level::K_SKELETON => Def { hp: 20.0, speed: 2.3, dmg: 8.0, reach: 15.0, cd: 2.2, windup: 0.4, radius: 0.4 },
        level::K_CREEPER => Def { hp: 20.0, speed: 3.2, dmg: 25.0, reach: 2.6, cd: 1.0, windup: 1.2, radius: 0.45 },
        level::K_SPIDER => Def { hp: 15.0, speed: 5.2, dmg: 6.0, reach: 1.5, cd: 0.85, windup: 0.3, radius: 0.6 },
        level::K_VINDICATOR => Def { hp: 80.0, speed: 4.4, dmg: 20.0, reach: 2.0, cd: 1.5, windup: 0.5, radius: 0.5 },
        level::K_ENCHANTER => Def { hp: 40.0, speed: 2.2, dmg: 8.0, reach: 16.0, cd: 3.0, windup: 0.6, radius: 0.45 },
        _ => Def { hp: 600.0, speed: 1.8, dmg: 15.0, reach: 9.0, cd: 3.2, windup: 0.6, radius: 0.55 },
    }
}

/// 0 veille, 1 poursuite, 2 préparation, 3 récupération, 4 mort, 5 mèche.
#[derive(Clone, Copy)]
pub struct Enemy {
    pub kind: u8,
    pub pos: Vec2,
    pub vel: Vec2,
    pub yaw: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub enchant: bool,
    pub state: u8,
    pub t: f32,
    pub cd: f32,
    pub walk: f32,
    pub flash: f32,
    pub dead_t: f32,
    pub summoned: bool,
    pub drops: bool,
}

impl Enemy {
    pub fn new(kind: u8, pos: Vec2, enchanted: bool) -> Self {
        let d = def(kind);
        let mult = if enchanted { 1.5 } else { 1.0 };
        Enemy {
            kind,
            pos,
            vel: Vec2::ZERO,
            yaw: 0.0,
            hp: d.hp * mult,
            max_hp: d.hp * mult,
            enchant: enchanted,
            state: 0,
            t: 0.0,
            cd: 0.5,
            walk: 0.0,
            flash: 0.0,
            dead_t: 0.0,
            summoned: false,
            drops: true,
        }
    }

    pub fn alive(&self) -> bool {
        self.state != 4
    }
}

impl World {
    pub fn step_enemy(&mut self, i: usize, dt: f32) {
        let kind = self.enemies[i].kind;
        let d = def(kind);
        let p = self.player.pos;
        let p_alive = self.player.state != 2;
        self.enemies[i].flash = (self.enemies[i].flash - dt).max(0.0);
        self.enemies[i].cd = (self.enemies[i].cd - dt).max(0.0);

        // Enchantement par proximité d'un enchanteur vivant (aura violette).
        if !self.enemies[i].enchant {
            let mut buffed = false;
            for en in &self.enemies {
                if en.kind == level::K_ENCHANTER
                    && en.alive()
                    && dist2(en.pos.x, en.pos.y, self.enemies[i].pos.x, self.enemies[i].pos.y) < 64.0
                {
                    buffed = true;
                    break;
                }
            }
            if buffed {
                self.enemies[i].enchant = true;
                self.enemies[i].max_hp *= 1.5;
                self.enemies[i].hp = (self.enemies[i].hp * 1.5).min(self.enemies[i].max_hp);
            }
        }
        if self.enemies[i].enchant && self.rng.chance(dt * 3.0) {
            let pos = self.enemies[i].pos;
            fx::aura(&mut self.particles, Vec3::new(pos.x, 1.0, pos.y));
        }

        if self.enemies[i].state == 4 {
            self.enemies[i].dead_t += dt;
            return;
        }

        let epos = self.enemies[i].pos;
        let dp = Vec2::new(p.x - epos.x, p.y - epos.y);
        let dist = dp.length();
        let state = self.enemies[i].state;

        match state {
            0 => {
                self.enemies[i].walk += dt * 1.2;
                if dist < 40.0 {
                    self.enemies[i].state = 1;
                }
            }
            1 => {
                if !p_alive {
                    self.enemies[i].vel *= 0.9;
                    self.enemies[i].walk += dt;
                    self.move_enemy(i, d.radius, dt);
                    return;
                }
                let dir = dp / dist.max(0.001);
                self.enemies[i].yaw = dir.y.atan2(dir.x);
                match kind {
                    level::K_SKELETON => {
                        let want = if dist > 13.0 {
                            dir * d.speed
                        } else if dist < 8.0 {
                            -dir * d.speed
                        } else {
                            Vec2::ZERO
                        };
                        let cur = self.enemies[i].vel;
                        self.enemies[i].vel += (want - cur) * (8.0 * dt).min(1.0);
                        if self.enemies[i].cd <= 0.0 && dist < d.reach {
                            self.enemies[i].state = 2;
                            self.enemies[i].t = 0.0;
                        }
                    }
                    level::K_ENCHANTER => {
                        let want = if dist < 9.0 {
                            -dir * d.speed
                        } else if dist > 14.0 {
                            dir * d.speed
                        } else {
                            Vec2::ZERO
                        };
                        let cur = self.enemies[i].vel;
                        self.enemies[i].vel += (want - cur) * (6.0 * dt).min(1.0);
                        if self.enemies[i].cd <= 0.0 && dist < d.reach {
                            self.enemies[i].state = 2;
                            self.enemies[i].t = 0.0;
                        }
                    }
                    level::K_CREEPER => {
                        let want = dir * d.speed;
                        let cur = self.enemies[i].vel;
                        self.enemies[i].vel += (want - cur) * (8.0 * dt).min(1.0);
                        if dist < d.reach {
                            self.enemies[i].state = 5;
                            self.enemies[i].t = 0.0;
                        }
                    }
                    _ => {
                        let want = dir * d.speed;
                        let cur = self.enemies[i].vel;
                        self.enemies[i].vel += (want - cur) * (8.0 * dt).min(1.0);
                        if dist < d.reach && self.enemies[i].cd <= 0.0 {
                            self.enemies[i].state = 2;
                            self.enemies[i].t = 0.0;
                        }
                    }
                }
                let spd = self.enemies[i].vel.length();
                self.enemies[i].walk += spd * dt * 1.6;
                self.move_enemy(i, d.radius, dt);
            }
            2 => {
                // Préparation d'attaque : on fait face, on frappe au bout du délai.
                self.enemies[i].t += dt;
                if p_alive {
                    self.enemies[i].yaw = dp.y.atan2(dp.x);
                }
                if self.enemies[i].t >= d.windup {
                    match kind {
                        level::K_SKELETON => {
                            let yaw = self.enemies[i].yaw;
                            let pos = self.enemies[i].pos;
                            let dir = Vec2::new(yaw.sin(), yaw.cos());
                            let dmg = d.dmg * if self.enemies[i].enchant { 1.25 } else { 1.0 };
                            self.projectiles.push(super::Projectile {
                                kind: PR_BONE,
                                pos: Vec3::new(pos.x + dir.x * 0.5, 1.2, pos.y + dir.y * 0.5),
                                vel: Vec3::new(dir.x * 17.0, 0.1, dir.y * 17.0),
                                dmg,
                                knock: 1.0,
                                ttl: 2.0,
                                target: None,
                            });
                        }
                        level::K_ENCHANTER => {
                            let yaw = self.enemies[i].yaw;
                            let pos = self.enemies[i].pos;
                            let dir = Vec2::new(yaw.sin(), yaw.cos());
                            let dmg = d.dmg * if self.enemies[i].enchant { 1.25 } else { 1.0 };
                            self.projectiles.push(super::Projectile {
                                kind: PR_ZAP,
                                pos: Vec3::new(pos.x + dir.x * 0.5, 1.4, pos.y + dir.y * 0.5),
                                vel: Vec3::new(dir.x * 11.0, 0.0, dir.y * 11.0),
                                dmg,
                                knock: 0.0,
                                ttl: 2.0,
                                target: None,
                            });
                            fx::aura(&mut self.particles, Vec3::new(pos.x, 1.6, pos.y));
                        }
                        _ => {
                            if p_alive && dist < d.reach + 0.6 {
                                let dmg = d.dmg * if self.enemies[i].enchant { 1.25 } else { 1.0 };
                                let pos = self.enemies[i].pos;
                                self.damage_player(dmg, pos);
                            }
                        }
                    }
                    self.enemies[i].cd = d.cd;
                    self.enemies[i].state = 3;
                    self.enemies[i].t = 0.0;
                }
            }
            3 => {
                self.enemies[i].t += dt;
                self.enemies[i].vel *= 0.85;
                self.move_enemy(i, d.radius, dt);
                if self.enemies[i].t > 0.35 {
                    self.enemies[i].state = 1;
                }
            }
            _ => {
                // Mèche du creeper : clignote puis explose.
                self.enemies[i].t += dt;
                self.enemies[i].vel *= 0.9;
                let t = self.enemies[i].t;
                let c = self.enemies[i].pos;
                if (t * 8.0) as i32 % 2 == 0 {
                    fx::poof(&mut self.particles, Vec3::new(c.x, 1.3, c.y), 1);
                }
                if t >= 1.2 {
                    fx::explosion(&mut self.particles, Vec3::new(c.x, 1.0, c.y));
                    self.shake = 0.35;
                    let d0 = dist2(p.x, p.y, c.x, c.y).sqrt();
                    if p_alive && d0 < 4.0 {
                        self.damage_player(25.0 * (1.0 - (d0 - 1.0) / 4.0).max(0.25), c);
                    }
                    for j in 0..self.enemies.len() {
                        if j != i && self.enemies[j].state != 4 {
                            let dj = dist2(c.x, c.y, self.enemies[j].pos.x, self.enemies[j].pos.y).sqrt();
                            if dj < 3.5 {
                                self.damage_enemy(j, 25.0 * (1.0 - dj / 3.5), c, 4.0);
                            }
                        }
                    }
                    self.enemies[i].state = 4;
                    self.enemies[i].dead_t = 1.2;
                    self.enemies[i].drops = false;
                    self.stats.kills += 1;
                }
            }
        }
    }

    fn move_enemy(&mut self, i: usize, r: f32, dt: f32) {
        let mut pos = self.enemies[i].pos;
        let vel = self.enemies[i].vel;
        World::collide_move(&self.level, &self.inters, &mut pos, vel, r, dt);
        self.enemies[i].pos = pos;
    }

    /// Nécromancien : invocations plafonnées à 7 zombies vivants, salves d'orbes
    /// repoussantes, téléportation si le héros se colle à lui, paliers à
    /// 75/50/25 % de vie (600 PV au total).
    pub fn step_boss(&mut self, i: usize, dt: f32) {
        let d = def(level::K_NECRO);
        let p = self.player.pos;
        self.enemies[i].flash = (self.enemies[i].flash - dt).max(0.0);
        if self.enemies[i].state == 4 {
            self.enemies[i].dead_t += dt;
            return;
        }
        let epos = self.enemies[i].pos;
        let dp = Vec2::new(p.x - epos.x, p.y - epos.y);
        let dist = dp.length();

        if !self.boss_active {
            self.enemies[i].walk += dt * 1.4;
            return;
        }

        // Paliers de vie (75 / 50 / 25 %).
        let phase = (((self.enemies[i].hp / self.enemies[i].max_hp) * 4.0).ceil() as u8).clamp(1, 4);
        if phase < self.boss_phase {
            self.boss_phase = phase;
            fx::summon_ring(&mut self.particles, Vec3::new(epos.x, 0.2, epos.y), 4.0);
            for k in 0..3u8 {
                let a = k as f32 * 2.09 + self.rng.range_f32(0.0, 1.0);
                self.spawn_summoned(level::K_ZOMBIE, Vec2::new(epos.x + a.cos() * 4.0, epos.y + a.sin() * 4.0));
            }
            self.teleport_boss(i);
            return;
        }

        // Invocation périodique (plafond : 7 zombies invoqués vivants).
        self.boss_cd_summon -= dt;
        if self.boss_cd_summon <= 0.0 {
            let alive = self.enemies.iter().filter(|o| o.summoned && o.alive()).count() as u8;
            if alive < 7 {
                fx::summon_ring(&mut self.particles, Vec3::new(epos.x, 0.2, epos.y), 4.0);
                let n = (7 - alive).min(4);
                for k in 0..n {
                    let a = k as f32 * 1.57 + self.rng.range_f32(0.0, 1.2);
                    self.spawn_summoned(level::K_ZOMBIE, Vec2::new(epos.x + a.cos() * 4.0, epos.y + a.sin() * 4.0));
                }
            }
            self.boss_cd_summon = 7.0;
        }

        // Salve d'orbes repoussantes.
        self.boss_cd_orb -= dt;
        if self.boss_cd_orb <= 0.0 && dist > 4.0 {
            self.boss_cd_orb = 3.4;
            let base = dp.y.atan2(dp.x);
            for k in [-0.25f32, 0.0, 0.25] {
                let a = base + k;
                let dir = Vec2::new(a.sin(), a.cos());
                self.projectiles.push(super::Projectile {
                    kind: PR_ORB,
                    pos: Vec3::new(epos.x + dir.x * 0.8, 1.3, epos.y + dir.y * 0.8),
                    vel: Vec3::new(dir.x * 7.5, 0.0, dir.y * 7.5),
                    dmg: d.dmg,
                    knock: 6.0,
                    ttl: 5.0,
                    target: None,
                });
            }
        }

        // Trop proche → téléportation dans l'arène.
        self.boss_cd_tele -= dt;
        if dist < 4.5 && self.boss_cd_tele <= 0.0 {
            self.teleport_boss(i);
        }

        // Déplacement lent vers le héros.
        if dist > 13.0 {
            let dir = dp / dist.max(0.001);
            let cur = self.enemies[i].vel;
            self.enemies[i].vel += (dir * d.speed - cur) * (4.0 * dt).min(1.0);
            self.enemies[i].walk += self.enemies[i].vel.length() * dt * 1.4;
            let mut pos = self.enemies[i].pos;
            let vel = self.enemies[i].vel;
            World::collide_move(&self.level, &self.inters, &mut pos, vel, d.radius, dt);
            self.enemies[i].pos = pos;
        } else {
            self.enemies[i].vel *= 0.9;
        }
        self.enemies[i].yaw = dp.y.atan2(dp.x);
    }

    fn spawn_summoned(&mut self, kind: u8, pos: Vec2) {
        let mut e = Enemy::new(kind, pos, false);
        e.summoned = true;
        self.enemies.push(e);
    }

    fn teleport_boss(&mut self, i: usize) {
        fx::poof(&mut self.particles, Vec3::new(self.enemies[i].pos.x, 1.2, self.enemies[i].pos.y), 8);
        self.boss_cd_tele = 1.5;
        // Reste dans l'arène du temple (tuiles 134..168 × 202..222).
        for _ in 0..12 {
            let a = self.rng.range_f32(0.0, TAU);
            let r = self.rng.range_f32(9.0, 15.0);
            let nx = (self.player.pos.x + a.cos() * r).clamp(134.0 * level::TILE, 168.0 * level::TILE);
            let nz = (self.player.pos.y + a.sin() * r).clamp(202.0 * level::TILE, 222.0 * level::TILE);
            if !self.level.solid_at(nx, nz) {
                self.enemies[i].pos = Vec2::new(nx, nz);
                break;
            }
        }
        let p = self.enemies[i].pos;
        fx::summon_ring(&mut self.particles, Vec3::new(p.x, 0.2, p.y), 3.0);
    }
}

/// Teinte de rendu d'un ennemi (flash de dégâts, enchantement violet).
pub fn tint(e: &Enemy) -> Vec4 {
    if e.flash > 0.0 {
        Vec4::new(2.4, 2.4, 2.4, 0.0)
    } else if e.enchant {
        Vec4::new(1.3, 0.85, 1.5, 0.0)
    } else {
        Vec4::ONE
    }
}
