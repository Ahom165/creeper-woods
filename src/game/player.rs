/// Héros : déplacement ZQSD/WASD (scancodes), mêlée à combo (mémorisé entre les
/// pauses — mécanique relevée chez les joueurs), arc chargé, roulade avec
/// i-frames, potion (75 % PV), trois artefacts, jauge d'âmes (plafond 300),
/// XP et enchantements.
use glam::{Vec2, Vec3};

use super::fx;
use super::{dist2, Input, World, B_1, B_2, B_3, B_E, B_SPACE};
use crate::level;

pub const SPEED: f32 = 8.0;
pub const RADIUS: f32 = 0.42;
pub const MELEE_BASE: f32 = 12.0;
pub const POTION_CD: f32 = 30.0;
pub const ART_CD: [f32; 3] = [12.0, 15.0, 5.0];
pub const SOUL_COST: f32 = 50.0;
pub const ENCH_NAMES: [&str; 3] = [
    "Tranchant : +25 % de dégâts de mêlée",
    "Puissance : +25 % de dégâts d'arc",
    "Robustesse : +25 PV maximum",
];

pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub yaw: f32,
    pub hp: f32,
    pub max_hp: f32,
    /// 0 actif, 1 roulade, 2 mort.
    pub state: u8,
    pub t_state: f32,
    pub roll_dir: Vec2,
    /// Étage du combo 0..3 — conservé tant que la fenêtre n'est pas écoulée,
    /// y compris pendant une pause (comportement source : avis joueurs).
    pub combo_stage: u8,
    pub combo_timer: f32,
    pub swing_t: f32,
    pub atk_cd: f32,
    pub roll_cd: f32,
    pub bow_charge: f32,
    pub bow_cd: f32,
    pub arrows: i32,
    pub emeralds: i32,
    pub souls: f32,
    pub xp: i32,
    pub level: i32,
    pub ench_points: i32,
    pub ench: [bool; 3],
    pub potion_cd: f32,
    pub art_cd: [f32; 3],
    pub walk: f32,
    pub hurt_flash: f32,
}

impl Player {
    pub fn new(pos: Vec2) -> Self {
        Self {
            pos,
            vel: Vec2::ZERO,
            yaw: 0.0,
            hp: 100.0,
            max_hp: 100.0,
            state: 0,
            t_state: 0.0,
            roll_dir: Vec2::Y,
            combo_stage: 0,
            combo_timer: 0.0,
            swing_t: 0.0,
            atk_cd: 0.0,
            roll_cd: 0.0,
            bow_charge: 0.0,
            bow_cd: 0.0,
            arrows: 25,
            emeralds: 0,
            souls: 0.0,
            xp: 0,
            level: 1,
            ench_points: 0,
            ench: [false; 3],
            potion_cd: 0.0,
            art_cd: [0.0; 3],
            walk: 0.0,
            hurt_flash: 0.0,
        }
    }
}

impl World {
    pub fn update_player(&mut self, input: &Input, dt: f32) {
        self.player.atk_cd = (self.player.atk_cd - dt).max(0.0);
        self.player.roll_cd = (self.player.roll_cd - dt).max(0.0);
        self.player.bow_cd = (self.player.bow_cd - dt).max(0.0);
        self.player.potion_cd = (self.player.potion_cd - dt).max(0.0);
        self.player.hurt_flash = (self.player.hurt_flash - dt).max(0.0);
        self.player.swing_t = (self.player.swing_t - dt * 3.2).max(0.0);
        for c in &mut self.player.art_cd {
            *c = (*c - dt).max(0.0);
        }
        self.player.combo_timer -= dt;
        if self.player.combo_timer <= 0.0 {
            self.player.combo_stage = 0;
        }
        if self.player.state == 2 {
            return;
        }

        // Orientation vers le curseur (visée MCD).
        let aim = input.aim - self.player.pos;
        if aim.length_squared() > 0.01 {
            self.player.yaw = aim.y.atan2(aim.x);
        }

        // Déplacement / roulade.
        if self.player.state == 1 {
            self.player.t_state += dt;
            let mut pos = self.player.pos;
            let vel = self.player.roll_dir * 11.5;
            World::collide_move(&self.level, &self.inters, &mut pos, vel, RADIUS, dt);
            self.player.pos = pos;
            if self.player.t_state % 0.12 < dt {
                fx::poof(&mut self.particles, Vec3::new(self.player.pos.x, 0.2, self.player.pos.y), 1);
            }
            if self.player.t_state >= 0.5 {
                self.player.state = 0;
            }
        } else {
            let want = if input.mv.length_squared() > 0.02 {
                input.mv.normalize_or_zero() * SPEED
            } else {
                Vec2::ZERO
            };
            self.player.vel += (want - self.player.vel) * (10.0 * dt).min(1.0);
            let mut pos = self.player.pos;
            let vel = self.player.vel;
            World::collide_move(&self.level, &self.inters, &mut pos, vel, RADIUS, dt);
            self.player.pos = pos;
            self.player.walk += self.player.vel.length() * dt * 1.55;
        }

        // Roulade (i-frames 0,42 s).
        if input.press & B_SPACE != 0 && self.player.state == 0 && self.player.roll_cd <= 0.0 {
            self.player.state = 1;
            self.player.t_state = 0.0;
            self.player.roll_cd = 1.0;
            self.player.roll_dir = if input.mv.length_squared() > 0.02 {
                input.mv.normalize_or_zero()
            } else {
                Vec2::new(self.player.yaw.sin(), self.player.yaw.cos())
            };
            self.player.combo_stage = 0;
            self.player.combo_timer = 0.0;
        }

        // Mêlée : combo 1-2-3, le 3e coup frappe plus fort et plus large.
        if input.lmb && self.player.state == 0 && self.player.atk_cd <= 0.0 {
            let stage3 = self.player.combo_stage >= 3
                || (self.player.combo_timer > 0.0 && self.player.combo_stage >= 2);
            self.player.combo_stage = if self.player.combo_timer > 0.0 {
                self.player.combo_stage % 3 + 1
            } else {
                1
            };
            self.player.swing_t = 1.0;
            self.player.combo_timer = 0.9;
            let finisher = self.player.combo_stage == 3;
            let dmg = MELEE_BASE
                * if self.player.ench[0] { 1.25 } else { 1.0 }
                * if finisher { 1.7 } else { 1.0 };
            let reach = if finisher { 2.9 } else { 2.5 };
            let arc = if finisher { 70.0f32.to_radians() } else { 55.0f32.to_radians() };
            let (px, py, yaw) = (self.player.pos.x, self.player.pos.y, self.player.yaw);
            let dir = Vec2::new(yaw.sin(), yaw.cos());
            let mut hits: Vec<usize> = Vec::new();
            for i in 0..self.enemies.len() {
                let e = &self.enemies[i];
                if e.state == 4 {
                    continue;
                }
                let to = Vec2::new(e.pos.x - px, e.pos.y - py);
                if to.length() < reach + 0.5 && to.normalize_or_zero().dot(dir) > arc.cos() {
                    hits.push(i);
                }
            }
            let from = self.player.pos;
            for i in hits {
                self.damage_enemy(i, dmg, from, if finisher { 3.2 } else { 2.0 });
            }
            if stage3 || finisher {
                let pos = Vec3::new(px + dir.x * 1.4, 1.2, py + dir.y * 1.4);
                fx::spark(&mut self.particles, pos, 3);
            }
            self.player.atk_cd = if finisher { 0.62 } else { 0.42 };
        }

        // Arc : maintenir pour charger, relâcher pour tirer.
        if input.rmb {
            self.player.bow_charge = (self.player.bow_charge + dt / 0.8).min(1.0);
        } else if input.rmb_release {
            if self.player.bow_charge > 0.12 {
                if self.player.arrows > 0 {
                    let (px, py, yaw) = (self.player.pos.x, self.player.pos.y, self.player.yaw);
                    let dir = Vec2::new(yaw.sin(), yaw.cos());
                    let dmg = (10.0 + 16.0 * self.player.bow_charge)
                        * if self.player.ench[1] { 1.25 } else { 1.0 };
                    self.projectiles.push(super::Projectile {
                        kind: super::PR_ARROW,
                        pos: Vec3::new(px + dir.x * 0.6, 1.15, py + dir.y * 0.6),
                        vel: Vec3::new(dir.x * 26.0, 0.35, dir.y * 26.0),
                        dmg,
                        knock: 1.5,
                        ttl: 2.2,
                        target: None,
                    });
                    self.player.arrows -= 1;
                    self.player.bow_cd = 0.3;
                } else if self.no_arrow_toast_cd <= 0.0 {
                    self.toast("Plus de flèches ! Ramassez des faisceaux au sol.");
                    self.no_arrow_toast_cd = 2.0;
                }
            }
            self.player.bow_charge = 0.0;
        }

        // Potion : soigne ~75 % des PV max (touche E).
        if input.press & B_E != 0 && self.player.potion_cd <= 0.0 && self.player.hp < self.player.max_hp {
            let heal = self.player.max_hp * 0.75;
            self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
            self.player.potion_cd = POTION_CD;
            let (px, py) = (self.player.pos.x, self.player.pos.y);
            self.float(
                Vec3::new(px, 2.0, py),
                format!("+{}", heal.round() as i32),
                [0.4, 1.0, 0.4, 1.0],
                24.0,
            );
            fx::soul_burst(&mut self.particles, Vec3::new(px, 1.2, py), 6);
        }

        // Artefact 1 : Cor du vent — repousse et blesse en cône.
        if input.press & B_1 != 0 && self.player.art_cd[0] <= 0.0 {
            self.player.art_cd[0] = ART_CD[0];
            let (px, py, yaw) = (self.player.pos.x, self.player.pos.y, self.player.yaw);
            let dir = Vec2::new(yaw.sin(), yaw.cos());
            fx::wind_cone(&mut self.particles, Vec3::new(px, 1.0, py), dir);
            let mut hits: Vec<usize> = Vec::new();
            for i in 0..self.enemies.len() {
                let e = &self.enemies[i];
                if e.state == 4 {
                    continue;
                }
                let to = Vec2::new(e.pos.x - px, e.pos.y - py);
                if to.length() < 6.5 && to.normalize_or_zero().dot(dir) > 50.0f32.to_radians().cos() {
                    hits.push(i);
                }
            }
            let from = self.player.pos;
            for i in hits {
                self.damage_enemy(i, 8.0, from, 5.0);
            }
        }

        // Artefact 2 : Flèches d'artifice — 3 fusées à tête chercheuse.
        if input.press & B_2 != 0 && self.player.art_cd[1] <= 0.0 {
            let (px, py) = (self.player.pos.x, self.player.pos.y);
            let mut targets: Vec<usize> = (0..self.enemies.len())
                .filter(|&i| {
                    self.enemies[i].state != 4
                        && dist2(px, py, self.enemies[i].pos.x, self.enemies[i].pos.y) < 26.0 * 26.0
                })
                .collect();
            targets.sort_by(|&a, &b| {
                let da = dist2(px, py, self.enemies[a].pos.x, self.enemies[a].pos.y);
                let db = dist2(px, py, self.enemies[b].pos.x, self.enemies[b].pos.y);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });
            targets.truncate(3);
            if targets.is_empty() {
                self.toast("Flèches d'artifice : aucune cible à portée !");
            } else {
                self.player.art_cd[1] = ART_CD[1];
                for ti in targets {
                    let tp = self.enemies[ti].pos;
                    let dir = (tp - Vec2::new(px, py)).normalize_or_zero();
                    self.projectiles.push(super::Projectile {
                        kind: super::PR_ROCKET,
                        pos: Vec3::new(px, 1.5, py),
                        vel: Vec3::new(dir.x * 14.0, 0.0, dir.y * 14.0),
                        dmg: 20.0,
                        knock: 3.0,
                        ttl: 3.0,
                        target: Some(ti),
                    });
                }
            }
        }

        // Artefact 3 : Guérison d'âme — consomme 50 âmes.
        if input.press & B_3 != 0 && self.player.art_cd[2] <= 0.0 {
            if self.player.souls >= SOUL_COST {
                self.player.souls -= SOUL_COST;
                self.player.art_cd[2] = ART_CD[2];
                let heal = self.player.max_hp * 0.3;
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                let (px, py) = (self.player.pos.x, self.player.pos.y);
                fx::soul_burst(&mut self.particles, Vec3::new(px, 1.2, py), 12);
                self.float(
                    Vec3::new(px, 2.0, py),
                    format!("+{}", heal.round() as i32),
                    [0.35, 1.0, 0.5, 1.0],
                    22.0,
                );
            } else if self.no_arrow_toast_cd <= 0.0 {
                self.toast("Pas assez d'âmes (50 requises) — éliminez des ennemis enchantés !");
                self.no_arrow_toast_cd = 2.0;
            }
        }

        // Filet de sécurité : les tuiles solides (eau/lave) n'auraient pas dû
        // laisser passer — dégâts de contact si cela arrive.
        if matches!(self.level.ground_at(self.player.pos.x, self.player.pos.y), level::G_LAVA | level::G_WATER) {
            self.player.hp -= 20.0 * dt;
            if self.player.hp <= 0.0 {
                self.player.hp = 0.0;
                self.player.state = 2;
                self.player_dead_t = 1.1;
            }
        }
    }

    /// Achat d'un enchantement dans l'inventaire (touches 1/2/3).
    pub fn buy_enchant(&mut self, slot: usize) {
        if self.player.ench[slot] || self.player.ench_points == 0 {
            return;
        }
        self.player.ench_points -= 1;
        self.player.ench[slot] = true;
        if slot == 2 {
            self.player.max_hp += 25.0;
            self.player.hp += 25.0;
        }
        let name = ENCH_NAMES[slot];
        self.toast(format!("Enchantement appliqué : {name}"));
    }
}
