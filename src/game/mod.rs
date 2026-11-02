/// Monde de jeu : joueur, ennemis, projectiles, ramassables, effets, objectifs.
/// Les équilibres suivent les données sourcées (wiki + critiques) : 100 PV,
/// 3 vies, jauge d'âmes plafonnée à 300, Nécromancien à 600 PV.
pub mod enemy;
pub mod fx;
pub mod player;

use glam::{Vec2, Vec3};
use std::f32::consts::TAU;

use crate::level::{self, Level, ZONES};
use crate::rng::Rng;
use fx::{FloatText, Particle, Pickup, Toast};

pub const SOULS_MAX: f32 = 300.0;
pub const LIVES_START: u8 = 3;
pub const SECRETS_TOTAL: u8 = 4;

// --- Entrées -------------------------------------------------------------------

pub struct Input {
    /// Direction de déplacement (-1..1).
    pub mv: Vec2,
    /// Point visé au sol (curseur).
    pub aim: Vec2,
    pub lmb: bool,
    pub rmb: bool,
    /// Bits d'appuis de la frame (voir B_*).
    pub press: u32,
    pub rmb_release: bool,
}

pub const B_ESC: u32 = 1 << 0;
pub const B_ENTER: u32 = 1 << 1;
pub const B_M: u32 = 1 << 2;
pub const B_I: u32 = 1 << 3;
pub const B_E: u32 = 1 << 4;
pub const B_SPACE: u32 = 1 << 5;
pub const B_1: u32 = 1 << 6;
pub const B_2: u32 = 1 << 7;
pub const B_3: u32 = 1 << 8;
pub const B_F: u32 = 1 << 9;
pub const B_Q: u32 = 1 << 10;

// --- Projectiles ---------------------------------------------------------------

pub const PR_ARROW: u8 = 0;
pub const PR_BONE: u8 = 1;
pub const PR_ZAP: u8 = 2;
pub const PR_ORB: u8 = 3;
pub const PR_ROCKET: u8 = 4;

pub struct Projectile {
    pub kind: u8,
    pub pos: Vec3,
    pub vel: Vec3,
    pub dmg: f32,
    pub knock: f32,
    pub ttl: f32,
    /// Cible homing (fusée d'artifice).
    pub target: Option<usize>,
}

// --- Interactables dynamiques ---------------------------------------------------

#[derive(Clone, Copy)]
pub struct Inter {
    pub kind: u8,
    pub x: f32,
    pub z: f32,
    pub data: u8,
    pub used: bool,
    pub anim: f32,
}

// --- Objectifs -------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjStep {
    Villager0,
    Caravan,
    Villagers,
    Tomb,
    Boss,
    Escape,
}

impl ObjStep {
    pub fn text(self, freed: u8) -> String {
        match self {
            ObjStep::Villager0 => "Libérez le villageois".to_string(),
            ObjStep::Caravan => "Trouvez la caravane".to_string(),
            ObjStep::Villagers => format!("Libérez les villageois ({freed}/5)"),
            ObjStep::Tomb => "Ouvrez la tombe runique".to_string(),
            ObjStep::Boss => "Vainquez le Nécromancien".to_string(),
            ObjStep::Escape => "Fuyez le temple".to_string(),
        }
    }

    /// Zone vers laquelle pointer sur la carte.
    pub fn target_zone(self) -> usize {
        match self {
            ObjStep::Villager0 => level::Z_VILLAGE,
            ObjStep::Caravan => level::Z_AUBERGE,
            ObjStep::Villagers => level::Z_VILLAGE,
            ObjStep::Tomb => level::Z_TOMBE,
            ObjStep::Boss => level::Z_TEMPLE,
            ObjStep::Escape => level::Z_TEMPLE,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Stats {
    pub kills: u32,
    pub time: f32,
}

pub struct World {
    pub level: Level,
    pub rng: Rng,
    pub time: f32,
    pub player: player::Player,
    pub enemies: Vec<enemy::Enemy>,
    pub spawn_taken: Vec<bool>,
    pub inters: Vec<Inter>,
    pub projectiles: Vec<Projectile>,
    pub pickups: Vec<Pickup>,
    pub particles: Vec<Particle>,
    pub floats: Vec<FloatText>,
    pub toasts: Vec<Toast>,
    pub obj: ObjStep,
    pub freed: u8,
    pub intro_done: bool,
    pub visited: [bool; 10],
    pub checkpoint: Vec2,
    pub lives: u8,
    pub stats: Stats,
    pub secrets: u8,
    pub boss_active: bool,
    pub boss_cd_summon: f32,
    pub boss_cd_orb: f32,
    pub boss_cd_tele: f32,
    pub boss_phase: u8,
    pub player_dead_t: f32,
    pub escaped: bool,
    pub hurt_flash: f32,
    pub shake: f32,
    pub no_arrow_toast_cd: f32,
}

impl World {
    pub fn new() -> Self {
        let level = Level::build();
        let inters = level
            .inters
            .iter()
            .map(|d| Inter { kind: d.kind, x: d.x, z: d.z, data: d.data, used: false, anim: 0.0 })
            .collect();
        let spawn = Vec2::new(level.spawn[0], level.spawn[1]);
        World {
            rng: Rng::new(0xC0FF_EE00_2020_0520),
            time: 0.0,
            player: player::Player::new(spawn),
            enemies: Vec::new(),
            spawn_taken: vec![false; level.spawners.len()],
            inters,
            projectiles: Vec::new(),
            pickups: Vec::new(),
            particles: Vec::new(),
            floats: Vec::new(),
            toasts: Vec::new(),
            obj: ObjStep::Villager0,
            freed: 0,
            intro_done: false,
            visited: [false; 10],
            checkpoint: spawn,
            lives: LIVES_START,
            stats: Stats::default(),
            secrets: 0,
            boss_active: false,
            boss_cd_summon: 4.0,
            boss_cd_orb: 2.0,
            boss_cd_tele: 0.0,
            boss_phase: 4,
            player_dead_t: 0.0,
            escaped: false,
            hurt_flash: 0.0,
            shake: 0.0,
            no_arrow_toast_cd: 0.0,
            level,
        }
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(Toast { text: text.into(), ttl: 4.5 });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn float(&mut self, pos: Vec3, text: impl Into<String>, color: [f32; 4], size: f32) {
        self.floats.push(FloatText { pos, text: text.into(), color, ttl: 0.9, size });
        if self.floats.len() > 40 {
            self.floats.remove(0);
        }
    }

    pub fn boss(&self) -> Option<&enemy::Enemy> {
        self.enemies.iter().find(|e| e.kind == level::K_NECRO && e.state != 4)
    }

    // --- Collision ---------------------------------------------------------------

    fn blocked(level: &Level, inters: &[Inter], x: f32, z: f32, r: f32) -> bool {
        if level.solid_at(x - r, z - r)
            || level.solid_at(x + r, z - r)
            || level.solid_at(x - r, z + r)
            || level.solid_at(x + r, z + r)
        {
            return true;
        }
        for (cx, cz, cr) in level.circles_near(x, z) {
            let dx = x - cx;
            let dz = z - cz;
            if dx * dx + dz * dz < (r + cr) * (r + cr) {
                return true;
            }
        }
        for inter in inters {
            if (inter.x - x).abs() > 12.0 || (inter.z - z).abs() > 12.0 {
                continue;
            }
            match inter.kind {
                level::I_VILLAGER | level::I_MERCHANT if !inter.used => {
                    if dist2(x, z, inter.x, inter.z) < 1.1 * 1.1 {
                        return true;
                    }
                }
                level::I_CHEST if !inter.used => {
                    if dist2(x, z, inter.x, inter.z) < 0.8 * 0.8 {
                        return true;
                    }
                }
                level::I_RUNEGATE if !inter.used => {
                    // Trois cercles le long de la porte.
                    for off in [-2.4f32, 0.0, 2.4] {
                        let (gx, gz) = if inter.data == 0 {
                            (inter.x, inter.z + off)
                        } else {
                            (inter.x + off, inter.z)
                        };
                        if dist2(x, z, gx, gz) < 1.5 * 1.5 {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Déplacement avec glissement le long des obstacles.
    pub fn collide_move(level: &Level, inters: &[Inter], pos: &mut Vec2, vel: Vec2, r: f32, dt: f32) {
        let nx = pos.x + vel.x * dt;
        if !Self::blocked(level, inters, nx, pos.y, r) {
            pos.x = nx;
        }
        let nz = pos.y + vel.y * dt;
        if !Self::blocked(level, inters, pos.x, nz, r) {
            pos.y = nz;
        }
    }

    // --- Dégâts ------------------------------------------------------------------

    pub fn damage_enemy(&mut self, i: usize, dmg: f32, from: Vec2, knock: f32) {
        if i >= self.enemies.len() || self.enemies[i].state == 4 {
            return;
        }
        self.enemies[i].hp -= dmg;
        self.enemies[i].flash = 0.15;
        let dir = (self.enemies[i].pos - from).normalize_or_zero();
        self.enemies[i].vel += dir * knock * 4.0;
        let center = Vec3::new(self.enemies[i].pos.x, 1.2, self.enemies[i].pos.y);
        fx::spark(&mut self.particles, center, 4);
        let crit = dmg >= 24.0;
        self.float(
            center + Vec3::new(0.0, 0.6, 0.0),
            format!("{}", dmg.round() as i32),
            if crit { [1.0, 0.85, 0.2, 1.0] } else { [1.0, 1.0, 1.0, 1.0] },
            if crit { 26.0 } else { 19.0 },
        );
        if self.enemies[i].hp <= 0.0 {
            self.kill_enemy(i);
        }
    }

    fn kill_enemy(&mut self, i: usize) {
        let e = &mut self.enemies[i];
        e.state = 4;
        e.dead_t = 0.0;
        self.stats.kills += 1;
        let pos2 = e.pos;
        let kind = e.kind;
        let enchant = e.enchant;
        let boss = kind == level::K_NECRO;
        let (xp, souls, em): (i32, f32, f32) = match kind {
            level::K_ZOMBIE => (3, 10.0, 2.0),
            level::K_SKELETON => (3, 10.0, 2.0),
            level::K_CREEPER => (4, 12.0, 3.0),
            level::K_SPIDER => (2, 8.0, 2.0),
            level::K_VINDICATOR => (12, 25.0, 6.0),
            level::K_ENCHANTER => (15, 30.0, 5.0),
            _ => (60, 100.0, 25.0),
        };
        let mult = if enchant { 2.0 } else { 1.0 };
        let center = Vec3::new(pos2.x, 1.0, pos2.y);
        fx::poof(&mut self.particles, center, 8);
        fx::drop_pickup(&mut self.pickups, fx::PK_SOUL, pos2, souls * mult, &mut self.rng);
        if enchant {
            fx::soul_burst(&mut self.particles, center, 10);
        }
        let n_em = self.rng.range_usize(1, 3) as f32 * em.max(1.0);
        fx::drop_pickup(&mut self.pickups, fx::PK_EMERALD, pos2, n_em, &mut self.rng);
        if self.rng.chance(0.3) {
            fx::drop_pickup(&mut self.pickups, fx::PK_ARROWS, pos2, self.rng.range_f32(2.0, 5.0), &mut self.rng);
        }
        if self.rng.chance(0.12) {
            fx::drop_pickup(&mut self.pickups, fx::PK_APPLE, pos2, 10.0, &mut self.rng);
        }
        self.add_xp((xp as f32 * mult) as i32);
        if boss {
            self.boss_active = false;
            fx::explosion(&mut self.particles, center);
            fx::soul_burst(&mut self.particles, center, 24);
            self.shake = 0.5;
            self.toast("Le Nécromancien s'effondre ! Le sceau du temple est brisé !");
        }
    }

    pub fn damage_player(&mut self, dmg: f32, from: Vec2) {
        if self.player.state == 2 {
            return;
        }
        // Invincibilité de la roulade (i-frames, mécanique MCD).
        if self.player.state == 1 && self.player.t_state < 0.42 {
            let (px, py) = (self.player.pos.x, self.player.pos.y);
            self.float(Vec3::new(px, 1.8, py), "Esquive !", [0.5, 0.9, 1.0, 1.0], 16.0);
            return;
        }
        self.player.hp -= dmg * 0.85; // armure fixe
        self.player.hurt_flash = 0.4;
        self.hurt_flash = 0.55;
        self.player.combo_stage = 0;
        self.player.combo_timer = 0.0;
        let dir = (self.player.pos - from).normalize_or_zero();
        self.player.vel += dir * 3.0;
        let (px, py) = (self.player.pos.x, self.player.pos.y);
        fx::spark(&mut self.particles, Vec3::new(px, 1.3, py), 5);
        self.float(
            Vec3::new(px, 2.0, py),
            format!("-{}", dmg.round() as i32),
            [1.0, 0.35, 0.3, 1.0],
            22.0,
        );
        if self.player.hp <= 0.0 {
            self.player.hp = 0.0;
            self.player.state = 2;
            self.player_dead_t = 1.1;
            fx::poof(&mut self.particles, Vec3::new(px, 1.0, py), 14);
        }
    }

    pub fn add_xp(&mut self, n: i32) {
        self.player.xp += n;
        loop {
            let need = 25 + 15 * (self.player.level - 1);
            if self.player.xp < need {
                break;
            }
            self.player.xp -= need;
            self.player.level += 1;
            self.player.ench_points += 1;
            let lvl = self.player.level;
            let (px, py) = (self.player.pos.x, self.player.pos.y);
            self.toast(format!("Niveau {lvl} atteint ! +1 point d'enchantement"));
            fx::soul_burst(
                &mut self.particles,
                Vec3::new(px, 1.2, py),
                8,
            );
        }
    }

    /// Réapparition après mort : retour au point de contrôle, vies restantes.
    pub fn respawn(&mut self) -> bool {
        if self.lives == 0 {
            return false;
        }
        self.lives -= 1;
        let cp = self.checkpoint;
        self.enemies.retain(|e| {
            (e.pos - cp).length() > 14.0 || e.state == 4
        });
        let p = &mut self.player;
        p.pos = cp;
        p.vel = Vec2::ZERO;
        p.hp = p.max_hp;
        p.state = 0;
        p.combo_stage = 0;
        self.player_dead_t = 0.0;
        self.hurt_flash = 0.0;
        true
    }

    /// Recommence la mission de zéro.
    pub fn retry(&mut self) {
        *self = World::new();
    }

    // --- Interaction ---------------------------------------------------------------

    /// Interactable utilisable le plus proche (≤ 2,6 m).
    pub fn interact_target(&self) -> Option<usize> {
        let p = &self.player;
        let mut best: Option<(f32, usize)> = None;
        for (i, inter) in self.inters.iter().enumerate() {
            if inter.used || inter.kind == level::I_RUNEGATE || inter.kind == level::I_BOSS {
                continue;
            }
            let d = dist2(p.pos.x, p.pos.y, inter.x, inter.z);
            if d < 2.6 * 2.6 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, i));
            }
        }
        best.map(|(_, i)| i)
    }

    pub fn interact_label(&self, i: usize) -> &'static str {
        match self.inters[i].kind {
            level::I_VILLAGER => "[F] Libérer le villageois",
            level::I_MERCHANT => "[F] Délivrer le marchand",
            level::I_CHEST => "[F] Ouvrir le coffre",
            level::I_BUTTON => "[F] Appuyer sur le bouton",
            level::I_PARCH => "[F] Prendre le parchemin",
            level::I_EXIT => {
                if self.obj == ObjStep::Escape {
                    "[F] S'échapper !"
                } else {
                    "Porte scellée…"
                }
            }
            _ => "",
        }
    }

    pub fn try_interact(&mut self) {
        let Some(i) = self.interact_target() else { return };
        let inter = self.inters[i];
        match inter.kind {
            level::I_VILLAGER => {
                self.inters[i].used = true;
                self.inters[i].anim = 0.001;
                if inter.data == 0 {
                    self.intro_done = true;
                    self.toast("Villageois libéré ! La caravane a disparu… retrouvez-la !");
                } else {
                    self.freed += 1;
                    let freed = self.freed;
                    self.toast(format!("Villageois libéré ! ({freed}/5)"));
                    self.float(
                        Vec3::new(inter.x, 2.0, inter.z),
                        "+20 XP",
                        [0.6, 1.0, 0.5, 1.0],
                        20.0,
                    );
                    self.add_xp(20);
                    // Vague de defense : les monstres accourent.
                    let n = 3 + self.rng.range_usize(0, 2);
                    for _ in 0..n {
                        let a = self.rng.range_f32(0.0, TAU);
                        let kind = if self.rng.chance(0.7) { level::K_ZOMBIE } else { level::K_SKELETON };
                        self.spawn_enemy(kind, inter.x + a.cos() * 5.0, inter.z + a.sin() * 5.0, true);
                    }
                }
                fx::soul_burst(&mut self.particles, Vec3::new(inter.x, 1.2, inter.z), 8);
            }
            level::I_MERCHANT => {
                self.inters[i].used = true;
                self.inters[i].anim = 0.001;
                self.secrets += 1;
                self.player.arrows += 25;
                self.player.ench_points += 1;
                self.toast("Marchand libéré : +25 flèches et +1 point d'enchantement !");
                fx::soul_burst(&mut self.particles, Vec3::new(inter.x, 1.2, inter.z), 10);
            }
            level::I_CHEST => {
                self.inters[i].used = true;
                self.inters[i].anim = 0.001;
                self.secrets += 1;
                self.player.arrows += 15;
                self.player.emeralds += 10;
                for _ in 0..2 {
                    fx::drop_pickup(&mut self.pickups, fx::PK_APPLE, Vec2::new(inter.x + 0.5, inter.z + 0.5), 10.0, &mut self.rng);
                }
                self.toast("Coffre de ravitaillement : +15 flèches, +10 émeraudes !");
            }
            level::I_BUTTON => {
                self.inters[i].used = true;
                self.open_gate(0);
                self.toast("Un déclic résonne… la porte runique s'ouvre !");
            }
            level::I_PLATE => {
                self.inters[i].used = true;
                self.open_gate(1);
                self.toast("La plaque s'enfonce… une crypte s'ouvre au nord du cimetière !");
            }
            level::I_PARCH => {
                self.inters[i].used = true;
                self.secrets += 1;
                self.player.ench_points += 1;
                self.toast("Parchemin secret ! +1 point d'enchantement");
            }
            level::I_EXIT => {
                if self.obj == ObjStep::Escape {
                    self.escaped = true;
                } else {
                    self.toast("La porte est scellée par la magie du Nécromancien.");
                }
            }
            _ => {}
        }
    }

    fn open_gate(&mut self, data: u8) {
        for inter in &mut self.inters {
            if inter.kind == level::I_RUNEGATE && inter.data == data && !inter.used {
                inter.used = true;
                inter.anim = 0.001;
            }
        }
    }

    pub fn spawn_enemy(&mut self, kind: u8, x: f32, z: f32, enchanted: bool) {
        self.enemies.push(enemy::Enemy::new(kind, Vec2::new(x, z), enchanted));
    }

    // --- Boucle ---------------------------------------------------------------------

    pub fn update(&mut self, input: &Input, dt: f32) {
        self.time += dt;
        self.stats.time += dt;
        self.no_arrow_toast_cd = (self.no_arrow_toast_cd - dt).max(0.0);

        self.wake_spawns();
        self.update_player(input, dt);

        // Séparation simple entre ennemis (avant les déplacements).
        let n = self.enemies.len();
        let mut sep = vec![Vec2::ZERO; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let (pi, pj) = (self.enemies[i].pos, self.enemies[j].pos);
                let d = pj - pi;
                let len = d.length();
                if len < 0.95 && len > 0.0001 {
                    let push = d / len * (0.95 - len) * 0.5;
                    sep[i] -= push;
                    sep[j] += push;
                }
            }
        }
        for (i, e) in self.enemies.iter_mut().enumerate() {
            e.pos += sep[i] * dt * 6.0;
        }

        for i in 0..self.enemies.len() {
            if self.enemies[i].kind == level::K_NECRO {
                self.step_boss(i, dt);
            } else {
                self.step_enemy(i, dt);
            }
        }
        self.enemies
            .retain(|e| !(e.state == 4 && e.dead_t > 0.55));

        self.update_projectiles(dt);
        self.update_pickups(dt);
        self.update_fx(dt);
        self.auto_triggers();
        self.check_zone();
        self.check_objectives();
        self.hurt_flash = (self.hurt_flash - dt * 1.6).max(0.0);
        self.shake = (self.shake - dt * 2.0).max(0.0);
    }

    fn wake_spawns(&mut self) {
        let p = self.player.pos;
        let mut wake: Vec<usize> = Vec::new();
        for (i, def) in self.level.spawners.iter().enumerate() {
            if !self.spawn_taken[i] && dist2(p.x, p.y, def.x, def.z) < 42.0 * 42.0 {
                wake.push(i);
            }
        }
        for i in wake {
            self.spawn_taken[i] = true;
            let def = self.level.spawners[i];
            // Les ennemis enchantés deviennent plus fréquents en fin de mission.
            let p_ench = match self.level.zone_at(def.x, def.z) {
                Some(6..=9) => 0.22,      // village, cimetière, tombe, temple
                Some(4..=5) => 0.12,      // creepers, rivière
                _ => 0.05,                // lac, bois, grotte, auberge, hors zone
            };
            let enchanted = self.rng.chance(p_ench);
            self.spawn_enemy(def.kind, def.x, def.z, enchanted);
        }
    }

    fn auto_triggers(&mut self) {
        // Plaque de pression : déclenchement au pas.
        let p = self.player.pos;
        for i in 0..self.inters.len() {
            if self.inters[i].kind == level::I_PLATE
                && !self.inters[i].used
                && dist2(p.x, p.y, self.inters[i].x, self.inters[i].z) < 1.4 * 1.4
            {
                self.inters[i].used = true;
                self.open_gate(1);
                self.toast("La plaque s'enfonce… une crypte s'ouvre au nord du cimetière !");
            }
        }
        // Réveil du boss à l'entrée du temple.
        if !self.boss_active
            && self.boss().is_some()
            && dist2(p.x, p.y, 150.0 * level::TILE, 206.0 * level::TILE) < 8.0 * 8.0
        {
            self.boss_active = true;
            self.toast("Le Nécromancien vous attend…");
        }
    }

    fn check_zone(&mut self) {
        let p = self.player.pos;
        if let Some(z) = self.level.zone_at(p.x, p.y) {
            if !self.visited[z] {
                self.visited[z] = true;
                let anchor = ZONES[z].anchor;
                self.checkpoint = Vec2::new(anchor[0] as f32 * level::TILE, anchor[1] as f32 * level::TILE);
                self.toast(format!("— {} —", ZONES[z].name));
            }
        }
    }

    fn check_objectives(&mut self) {
        let p = self.player.pos;
        match self.obj {
            ObjStep::Villager0 => {
                if self.intro_done {
                    self.obj = ObjStep::Caravan;
                    self.toast("Nouvel objectif : retrouvez la caravane abandonnée");
                }
            }
            ObjStep::Caravan => {
                let r = self.level.caravan_rect;
                let tx = p.x / level::TILE;
                let tz = p.y / level::TILE;
                if tx >= r[0] as f32 && tz >= r[1] as f32 && tx <= r[2] as f32 && tz <= r[3] as f32 {
                    self.obj = ObjStep::Villagers;
                    self.toast("La caravane est saccagée ! Libérez les villageois (0/5)");
                }
            }
            ObjStep::Villagers => {
                if self.freed >= 5 {
                    self.obj = ObjStep::Tomb;
                    self.toast("Tous les villageois sont libres ! Ouvrez la tombe runique");
                }
            }
            ObjStep::Tomb => {
                if self.inters.iter().any(|i| i.kind == level::I_RUNEGATE && i.data == 0 && i.used) {
                    self.obj = ObjStep::Boss;
                    self.toast("Le chemin du temple est ouvert… Vainquez le Nécromancien !");
                }
            }
            ObjStep::Boss => {
                if self.boss().is_none() {
                    self.obj = ObjStep::Escape;
                    self.toast("Fuyez le temple par le portail !");
                }
            }
            ObjStep::Escape => {}
        }
    }

    fn update_projectiles(&mut self, dt: f32) {
        let player_pos = self.player.pos;
        let player_alive = self.player.state != 2;
        let n = self.projectiles.len();
        for pi in 0..n {
            let kind = self.projectiles[pi].kind;
            self.projectiles[pi].ttl -= dt;
            if kind == PR_ROCKET {
                if let Some(ti) = self.projectiles[pi].target {
                    if ti < self.enemies.len() && self.enemies[ti].state != 4 {
                        let (ex, ez) = (self.enemies[ti].pos.x, self.enemies[ti].pos.y);
                        let cur = self.projectiles[pi].vel.normalize_or_zero();
                        let want = Vec3::new(ex - self.projectiles[pi].pos.x, 1.1 - self.projectiles[pi].pos.y, ez - self.projectiles[pi].pos.z).normalize_or_zero();
                        self.projectiles[pi].vel = (cur + want * dt * 6.0).normalize_or_zero() * 14.0;
                    }
                }
                fx::rocket_trail(&mut self.particles, self.projectiles[pi].pos);
            }
            let vel = self.projectiles[pi].vel;
            self.projectiles[pi].pos += vel * dt;
            if self.projectiles[pi].ttl <= 0.0 {
                continue;
            }
            let pr_pos = self.projectiles[pi].pos;
            if self.level.solid_at(pr_pos.x, pr_pos.z) {
                self.projectiles[pi].ttl = 0.0;
                fx::spark(&mut self.particles, pr_pos, 3);
                continue;
            }
            match kind {
                PR_ARROW => {
                    let mut hit: Option<usize> = None;
                    for i in 0..self.enemies.len() {
                        let e = &self.enemies[i];
                        if e.state != 4 && dist2(pr_pos.x, pr_pos.z, e.pos.x, e.pos.y) < 0.64 && pr_pos.y < 2.2 {
                            hit = Some(i);
                            break;
                        }
                    }
                    if let Some(i) = hit {
                        let c = Vec2::new(pr_pos.x, pr_pos.z);
                        let dmg = self.projectiles[pi].dmg;
                        let knock = self.projectiles[pi].knock;
                        self.damage_enemy(i, dmg, c, knock);
                        self.projectiles[pi].ttl = 0.0;
                    }
                }
                PR_ROCKET => {
                    let mut hit = false;
                    for e in &self.enemies {
                        if e.state != 4 && dist2(pr_pos.x, pr_pos.z, e.pos.x, e.pos.y) < 1.0 {
                            hit = true;
                            break;
                        }
                    }
                    if hit {
                        self.projectiles[pi].ttl = 0.0;
                        fx::explosion(&mut self.particles, pr_pos);
                        self.shake = 0.25;
                        let c = Vec2::new(pr_pos.x, pr_pos.z);
                        let dmg = self.projectiles[pi].dmg;
                        let knock = self.projectiles[pi].knock;
                        let targets: Vec<usize> = (0..self.enemies.len())
                            .filter(|&i| self.enemies[i].state != 4 && dist2(c.x, c.y, self.enemies[i].pos.x, self.enemies[i].pos.y) < 9.0)
                            .collect();
                        for i in targets {
                            self.damage_enemy(i, dmg, c, knock);
                        }
                    }
                }
                PR_BONE | PR_ZAP | PR_ORB
                    if player_alive
                        && dist2(pr_pos.x, pr_pos.z, player_pos.x, player_pos.y) < 0.85
                        && pr_pos.y < 2.0 =>
                {
                    let dmg = self.projectiles[pi].dmg;
                    let knock = self.projectiles[pi].knock;
                    self.damage_player(dmg, Vec2::new(pr_pos.x, pr_pos.z));
                    if kind == PR_ORB {
                        let dir = (self.player.pos - Vec2::new(pr_pos.x, pr_pos.z)).normalize_or_zero();
                        self.player.vel += dir * knock;
                    }
                    self.projectiles[pi].ttl = 0.0;
                }
                _ => {}
            }
        }
        self.projectiles.retain(|pr| pr.ttl > 0.0);
    }

    fn update_pickups(&mut self, dt: f32) {
        let p = self.player.pos;
        let alive = self.player.state != 2;
        let mut collected: Vec<(u8, f32)> = Vec::new();
        for pk in &mut self.pickups {
            pk.t += dt;
            pk.ttl -= dt;
            let d = Vec2::new(p.x - pk.pos.x, p.y - pk.pos.y);
            let len = d.length();
            if alive && len < 3.2 {
                pk.vel += d / len.max(0.001) * 22.0 * dt;
            }
            pk.vel *= (1.0 - 3.0 * dt).max(0.0);
            pk.pos += pk.vel * dt;
            if alive && pk.ttl > 0.0 && dist2(p.x, p.y, pk.pos.x, pk.pos.y) < 0.9 * 0.9 {
                collected.push((pk.kind, pk.amount));
                pk.ttl = 0.0;
            }
        }
        self.pickups.retain(|pk| pk.ttl > 0.0);
        for (kind, amount) in collected {
            match kind {
                fx::PK_EMERALD => {
                    self.player.emeralds += amount.round() as i32;
                }
                fx::PK_ARROWS => {
                    self.player.arrows = (self.player.arrows + amount.round() as i32).min(60);
                }
                fx::PK_APPLE => {
                    self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                    let (px, py) = (self.player.pos.x, self.player.pos.y);
                    self.float(
                        Vec3::new(px, 1.9, py),
                        format!("+{}", amount as i32),
                        [0.5, 1.0, 0.5, 1.0],
                        18.0,
                    );
                }
                _ => {
                    self.player.souls = (self.player.souls + amount).min(SOULS_MAX);
                }
            }
        }
    }

    fn update_fx(&mut self, dt: f32) {
        for pt in &mut self.particles {
            pt.ttl -= dt;
            pt.pos += pt.vel * dt;
            pt.vel.y -= pt.grav * dt;
            if pt.pos.y < 0.05 && pt.grav > 0.0 {
                pt.vel.y = 0.0;
                pt.pos.y = 0.05;
            }
        }
        self.particles.retain(|p| p.alive());
        if self.particles.len() > 900 {
            self.particles.drain(..self.particles.len() - 900);
        }
        for f in &mut self.floats {
            f.ttl -= dt;
            f.pos.y += dt * 1.4;
        }
        self.floats.retain(|f| f.ttl > 0.0);
        for t in &mut self.toasts {
            t.ttl -= dt;
        }
        self.toasts.retain(|t| t.ttl > 0.0);
    }
}

pub fn dist2(x1: f32, z1: f32, x2: f32, z2: f32) -> f32 {
    let dx = x1 - x2;
    let dz = z1 - z2;
    dx * dx + dz * dz
}
