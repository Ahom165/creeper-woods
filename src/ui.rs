/// Interface 2D française : HUD de mission (cœur, âmes, artefacts, flèches,
/// émeraudes, XP, vies, objectif, barre de boss, toasts, dégâts flottants),
/// carte (M), inventaire (I) et écrans titre/pause/mort/échec/victoire.
use glam::Vec2;

use crate::app::Screen;
use crate::game::World;
use crate::game::player as pl;
use crate::gfx::camera::Camera;
use crate::gfx::textures as tex;
use crate::gfx::{text, Gfx, UiFrame};
use crate::level;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const GOLD: [f32; 4] = [0.98, 0.84, 0.34, 1.0];
const RED: [f32; 4] = [0.9, 0.25, 0.22, 1.0];
const GREEN: [f32; 4] = [0.42, 0.9, 0.45, 1.0];
const SOUL: [f32; 4] = [0.35, 0.95, 0.5, 1.0];
const PANEL: [f32; 4] = [0.06, 0.06, 0.1, 0.86];
const DIM: [f32; 4] = [0.02, 0.02, 0.05, 0.6];
const GREY: [f32; 4] = [0.75, 0.75, 0.78, 1.0];

/// Contexte figé pour une frame d'interface (découplé de l'App pour les emprunts).
pub struct Ctx<'a> {
    pub screen: Screen,
    pub world: &'a World,
    pub map_open: bool,
    pub inv_open: bool,
    pub app_time: f32,
    pub cursor: Vec2,
}

fn uv(tile: usize) -> (f32, f32, f32, f32) {
    let s = 1.0 / tex::GRID as f32;
    let u0 = (tile % tex::GRID) as f32 * s;
    let v0 = (tile / tex::GRID) as f32 * s;
    let inset = s * 0.02;
    (u0 + inset, v0 + inset, u0 + s - inset, v0 + s - inset)
}

fn quad(out: &mut Vec<text::UiVertex>, x: f32, y: f32, w: f32, h: f32, tile: usize, c: [f32; 4]) {
    let (u0, v0, u1, v1) = uv(tile);
    text::push_quad_raw(out, x, y, w, h, u0, v0, u1, v1, c);
}

#[allow(clippy::too_many_arguments)] // helper de dessin à paramètres alignés
fn bar(out: &mut Vec<text::UiVertex>, x: f32, y: f32, w: f32, h: f32, frac: f32, bg: [f32; 4], fg: [f32; 4]) {
    quad(out, x, y, w, h, tex::T_WHITE, bg);
    let fw = w * frac.clamp(0.0, 1.0);
    if fw > 0.5 {
        quad(out, x + 1.0, y + 1.0, (fw - 2.0).max(0.0), h - 2.0, tex::T_WHITE, fg);
    }
}

#[allow(clippy::too_many_arguments)] // helper de dessin à paramètres alignés
fn txt(g: &Gfx, out: &mut Vec<text::UiVertex>, s: &str, x: f32, y: f32, px: f32, c: [f32; 4], align: u32) {
    g.text.push_text(out, s, x, y, px, c, align);
}

/// Dessine toute l'interface de la frame.
pub fn draw(g: &Gfx, ui: &mut UiFrame, ctx: &Ctx, cam: &Camera) {
    let t = ctx.app_time;
    // Vignette plein écran (texture radiale dédiée).
    text::push_quad_raw(&mut ui.vignette, 0.0, 0.0, 1280.0, 720.0, 0.0, 0.0, 1.0, 1.0, [1.0, 1.0, 1.0, 1.0]);
    if std::env::var("MCD_REDTEST").is_ok() {
        let (u0, v0, u1, v1) = uv(tex::T_WHITE);
        text::push_quad_raw(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, u0, v0, u1, v1, [1.0, 0.0, 0.0, 1.0]);
    }

    match ctx.screen {
        Screen::Title => title(g, ui, t),
        Screen::Playing => {
            hud(g, ui, ctx, cam);
            if ctx.map_open {
                map(g, ui, ctx);
            } else if ctx.inv_open {
                inventory(g, ui, ctx);
            }
        }
        Screen::Paused => {
            hud(g, ui, ctx, cam);
            pause(g, ui);
        }
        Screen::Dead => dead(g, ui, ctx),
        Screen::GameOver => game_over(g, ui, ctx, t),
        Screen::Victory => victory(g, ui, ctx, t),
    }
}

fn title(g: &Gfx, ui: &mut UiFrame, t: f32) {
    quad(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, tex::T_WHITE, [0.03, 0.04, 0.09, 1.0]);
    txt(g, &mut ui.text, "MINECRAFT", 640.0, 190.0, 74.0, WHITE, 1);
    txt(g, &mut ui.text, "DUNGEONS", 640.0, 268.0, 74.0, GOLD, 1);
    txt(g, &mut ui.text, "— Creeper Woods —", 640.0, 330.0, 30.0, GREY, 1);
    txt(g, &mut ui.text, "Recréation libre en Rust + wgpu · HUD français", 640.0, 368.0, 20.0, GREY, 1);

    if (t * 1.6) as i32 % 2 == 0 {
        txt(g, &mut ui.text, "Appuyez sur ENTRÉE pour jouer", 640.0, 470.0, 28.0, GOLD, 1);
    }

    txt(g, &mut ui.text, "Déplacement : ZQSD / WASD    Mêlée : clic gauche    Arc : clic droit (maintenir)", 640.0, 560.0, 18.0, GREY, 1);
    txt(g, &mut ui.text, "Roulade : ESPACE    Potion : E    Artefacts : 1 2 3    Carte : M    Inventaire : I    Interagir : F", 640.0, 588.0, 18.0, GREY, 1);
    txt(g, &mut ui.text, "v0.1.0", 20.0, 690.0, 16.0, GREY, 0);
    txt(g, &mut ui.text, "Trois vies par mission · Âmes : plafond 300 · Nécromancien : 600 PV", 640.0, 690.0, 16.0, GREY, 1);
}

fn hud(g: &Gfx, ui: &mut UiFrame, ctx: &Ctx, cam: &Camera) {
    let w = &ctx.world;
    let p = &w.player;

    // Flash de dégâts + pouls PV bas.
    if w.hurt_flash > 0.01 {
        quad(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, tex::T_WHITE, [0.8, 0.05, 0.05, w.hurt_flash * 0.4]);
    }
    if p.hp < 25.0 && p.state != 2 {
        let a = 0.12 + (ctx.app_time * 5.0).sin().abs() * 0.10;
        quad(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, tex::T_WHITE, [0.7, 0.0, 0.0, a]);
    }

    // Objectif (haut-gauche).
    quad(&mut ui.icons, 16.0, 14.0, 340.0, 64.0, tex::T_WHITE, PANEL);
    txt(g, &mut ui.text, "Objectif", 28.0, 36.0, 15.0, GOLD, 0);
    txt(g, &mut ui.text, &w.obj.text(w.freed), 28.0, 62.0, 19.0, WHITE, 0);

    // Vies (haut-centre).
    for i in 0..w.lives {
        quad(&mut ui.icons, 612.0 + i as f32 * 34.0, 16.0, 26.0, 26.0, tex::T_LIFE, WHITE);
    }

    // Barre de boss.
    if let Some(b) = w.boss() {
        txt(g, &mut ui.text, "Nécromancien", 640.0, 66.0, 18.0, RED, 1);
        bar(&mut ui.icons, 390.0, 74.0, 500.0, 14.0, b.hp / b.max_hp, [0.1, 0.02, 0.02, 0.9], RED);
    }

    // Cœur + PV (bas-centre).
    quad(&mut ui.icons, 522.0, 634.0, 30.0, 30.0, tex::T_HEART, WHITE);
    bar(&mut ui.icons, 560.0, 641.0, 170.0, 16.0, p.hp / p.max_hp, [0.08, 0.05, 0.05, 0.9], [0.85, 0.2, 0.2, 1.0]);
    txt(
        g,
        &mut ui.text,
        &format!("{}/{}", p.hp.round() as i32, p.max_hp.round() as i32),
        742.0,
        655.0,
        17.0,
        WHITE,
        0,
    );

    // Âmes + artefacts (bas-gauche).
    quad(&mut ui.icons, 20.0, 620.0, 26.0, 26.0, tex::T_SOUL, WHITE);
    bar(&mut ui.icons, 54.0, 628.0, 140.0, 12.0, p.souls / crate::game::SOULS_MAX, [0.05, 0.1, 0.05, 0.9], SOUL);
    txt(
        g,
        &mut ui.text,
        &format!("Âmes {}/300", p.souls.round() as i32),
        54.0,
        612.0,
        14.0,
        GREY,
        0,
    );
    let arts = [tex::T_WINDHORN, tex::T_FIREWORK, tex::T_SOULHEAL];
    for (i, (tile, cooldown)) in arts.iter().zip(p.art_cd.iter()).enumerate() {
        let x = 210.0 + i as f32 * 62.0;
        quad(&mut ui.icons, x, 600.0, 52.0, 52.0, tex::T_WHITE, [0.05, 0.05, 0.09, 0.85]);
        quad(&mut ui.icons, x + 6.0, 606.0, 40.0, 40.0, *tile, WHITE);
        if *cooldown > 0.0 {
            let frac = *cooldown / pl::ART_CD[i];
            quad(&mut ui.icons, x + 3.0, 603.0, 46.0 * frac, 46.0, tex::T_WHITE, [0.0, 0.0, 0.0, 0.65]);
        }
        txt(g, &mut ui.text, &format!("{}", i + 1), x + 42.0, 648.0, 13.0, GOLD, 1);
        if i == 2 {
            txt(g, &mut ui.text, "50", x + 8.0, 648.0, 12.0, SOUL, 0);
        }
    }

    // Flèches / émeraudes / potion / XP (bas-droite).
    quad(&mut ui.icons, 1020.0, 646.0, 24.0, 24.0, tex::T_ARROW, WHITE);
    txt(g, &mut ui.text, &format!("{}", p.arrows), 1050.0, 664.0, 19.0, WHITE, 0);
    quad(&mut ui.icons, 1096.0, 646.0, 22.0, 22.0, tex::T_EMERALD, WHITE);
    txt(g, &mut ui.text, &format!("{}", p.emeralds), 1124.0, 664.0, 19.0, GREEN, 0);
    let pot = if p.potion_cd > 0.0 {
        format!("{:.0}s", p.potion_cd)
    } else {
        "prête".to_string()
    };
    quad(&mut ui.icons, 1180.0, 644.0, 24.0, 26.0, tex::T_POTION, WHITE);
    txt(g, &mut ui.text, &pot, 1210.0, 664.0, 16.0, GREY, 0);
    let need = 25 + 15 * (p.level - 1);
    txt(
        g,
        &mut ui.text,
        &format!("Niveau {} ({} pts d'ench.)", p.level, p.ench_points),
        1260.0,
        614.0,
        15.0,
        GOLD,
        2,
    );
    bar(&mut ui.icons, 1100.0, 620.0, 160.0, 8.0, p.xp as f32 / need as f32, [0.08, 0.08, 0.12, 0.9], GOLD);

    // Réticule + charge de l'arc + combo.
    let (cx, cy) = (ctx.cursor.x, ctx.cursor.y);
    quad(&mut ui.icons, cx - 11.0, cy - 11.0, 22.0, 22.0, tex::T_CROSSHAIR, WHITE);
    if p.bow_charge > 0.01 {
        bar(&mut ui.icons, cx - 30.0, cy + 18.0, 60.0, 6.0, p.bow_charge, [0.05, 0.05, 0.05, 0.8], GOLD);
    }
    if p.combo_timer > 0.0 && p.combo_stage >= 2 {
        txt(
            g,
            &mut ui.text,
            &format!("Combo ×{}", p.combo_stage),
            cx,
            cy - 34.0,
            16.0,
            GOLD,
            1,
        );
    }

    // Invite d'interaction.
    if let Some(i) = w.interact_target() {
        txt(g, &mut ui.text, w.interact_label(i), 640.0, 566.0, 20.0, GOLD, 1);
    }

    // Toasts (masqués sous la carte et l'inventaire).
    if ctx.map_open || ctx.inv_open {
        return;
    }
    let mut y = 520.0;
    for toast in &w.toasts {
        let a = (toast.ttl / 0.5).min(1.0);
        txt(g, &mut ui.text, &toast.text, 640.0, y, 19.0, [1.0, 0.97, 0.9, a], 1);
        y += 28.0;
    }

    // Dégâts flottants projetés au sol.
    for f in &w.floats {
        if let Some(ndc) = cam.project(f.pos) {
            let scr = g.ndc_to_design(ndc);
            let a = (f.ttl / 0.4).min(1.0);
            txt(g, &mut ui.text, &f.text, scr.x, scr.y, f.size, [f.color[0], f.color[1], f.color[2], a], 1);
        }
    }
}

fn map(g: &Gfx, ui: &mut UiFrame, ctx: &Ctx) {
    quad(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, tex::T_WHITE, DIM);
    let (px, py, size) = (360.0, 80.0, 560.0);
    quad(&mut ui.icons, px - 8.0, py - 8.0, size + 16.0, size + 16.0, tex::T_WHITE, PANEL);
    // Texture de la carte (240×240 px) étirée.
    text::push_quad_raw(&mut ui.map, px, py, size, size, 0.0, 0.0, 1.0, 1.0, WHITE);
    let scale = size / crate::level::MAP as f32;
    let dot = |ui: &mut UiFrame, wx: f32, wz: f32, c: [f32; 4], s: f32| {
        let x = px + wx / crate::level::TILE * scale;
        let y = py + wz / crate::level::TILE * scale;
        quad(&mut ui.icons, x - s * 0.5, y - s * 0.5, s, s, tex::T_WHITE, c);
    };
    for e in &ctx.world.enemies {
        if e.state != 4 {
            dot(ui, e.pos.x, e.pos.y, RED, 4.0);
        }
    }
    for inter in &ctx.world.inters {
        if inter.kind == level::I_VILLAGER && !inter.used {
            dot(ui, inter.x, inter.z, GREEN, 5.0);
        }
    }
    // Objectif courant + sortie.
    let target = ctx.world.obj.target_zone();
    let a = level::ZONES[target].anchor;
    dot(ui, a[0] as f32 * level::TILE, a[1] as f32 * level::TILE, GOLD, 8.0);
    if ctx.world.obj == crate::game::ObjStep::Escape {
        dot(ui, 150.0 * level::TILE, 224.0 * level::TILE, [0.7, 0.4, 1.0, 1.0], 8.0);
    }
    dot(ui, ctx.world.player.pos.x, ctx.world.player.pos.y, WHITE, 7.0);

    txt(g, &mut ui.text, "CARTE — Creeper Woods", 640.0, 56.0, 30.0, GOLD, 1);
    txt(g, &mut ui.text, "Vous (blanc) · Ennemis (rouge) · Villageois (vert) · Objectif (or) · Sortie (violet)", 640.0, 668.0, 17.0, GREY, 1);
    txt(g, &mut ui.text, "[M] ou [ÉCHAP] pour fermer", 640.0, 694.0, 16.0, GREY, 1);
}

fn inventory(g: &Gfx, ui: &mut UiFrame, ctx: &Ctx) {
    let p = &ctx.world.player;
    quad(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, tex::T_WHITE, DIM);
    let (x, y, w, h) = (280.0, 56.0, 720.0, 612.0);
    quad(&mut ui.icons, x, y, w, h, tex::T_WHITE, PANEL);
    txt(g, &mut ui.text, "INVENTAIRE", 640.0, 112.0, 34.0, GOLD, 1);

    let mut line = 160.0;
    let row = |g: &Gfx, ui: &mut UiFrame, line: &mut f32, a: &str, b: &str, ca: [f32; 4]| {
        txt(g, &mut ui.text, a, x + 30.0, *line, 19.0, ca, 0);
        txt(g, &mut ui.text, b, x + w - 30.0, *line, 19.0, WHITE, 2);
        *line += 32.0;
    };
    row(g, ui, &mut line, "Épée", &format!("Dégâts {}{}", pl::MELEE_BASE as i32, if p.ench[0] { " (+25 %)" } else { "" }), WHITE);
    row(g, ui, &mut line, "Arc", &format!("Dégâts 10–{} · flèches {}/60", (26.0 * if p.ench[1] { 1.25 } else { 1.0 }) as i32, p.arrows), WHITE);
    row(g, ui, &mut line, "Armure", "Réduction fixe 15 %", WHITE);
    row(g, ui, &mut line, "Points de vie", &format!("{}/{}", p.hp.round() as i32, p.max_hp.round() as i32), WHITE);
    row(g, ui, &mut line, "Âmes", &format!("{}/300", p.souls.round() as i32), SOUL);
    row(g, ui, &mut line, "Émeraudes", &format!("{}", p.emeralds), GREEN);
    let need = 25 + 15 * (p.level - 1);
    row(g, ui, &mut line, "Progression", &format!("Niveau {} · XP {}/{} · {} pts d'enchantement", p.level, p.xp, need, p.ench_points), GOLD);

    line += 14.0;
    txt(g, &mut ui.text, "Enchantements — [1][2][3] pour appliquer (1 point)", x + 30.0, line, 18.0, GOLD, 0);
    line += 30.0;
    for i in 0..3 {
        let state = if p.ench[i] {
            "Acquis".to_string()
        } else if p.ench_points > 0 {
            "Prix : 1 pt".to_string()
        } else {
            "Verrouillé".to_string()
        };
        let c = if p.ench[i] { GREEN } else if p.ench_points > 0 { GOLD } else { GREY };
        txt(g, &mut ui.text, &format!("[{}]", i + 1), x + 40.0, line, 19.0, GOLD, 0);
        txt(g, &mut ui.text, pl::ENCH_NAMES[i], x + 90.0, line, 19.0, WHITE, 0);
        txt(g, &mut ui.text, &state, x + w - 40.0, line, 19.0, c, 2);
        line += 30.0;
    }

    line += 14.0;
    txt(g, &mut ui.text, "Artefacts", x + 30.0, line, 18.0, GOLD, 0);
    line += 30.0;
    let arts = [
        ("1 · Cor du vent", p.art_cd[0]),
        ("2 · Flèches d'artifice", p.art_cd[1]),
        ("3 · Guérison d'âme (50 âmes)", p.art_cd[2]),
    ];
    for (name, cd) in arts {
        let state = if cd > 0.0 { format!("{:.0}s", cd) } else { "prêt".to_string() };
        row(g, ui, &mut line, name, &state, WHITE);
    }

    txt(g, &mut ui.text, "[I] Fermer", 640.0, y + h - 22.0, 17.0, GREY, 1);
}

fn pause(g: &Gfx, ui: &mut UiFrame) {
    quad(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, tex::T_WHITE, DIM);
    txt(g, &mut ui.text, "PAUSE", 640.0, 250.0, 56.0, GOLD, 1);
    txt(g, &mut ui.text, "ÉCHAP — Reprendre", 640.0, 350.0, 24.0, WHITE, 1);
    txt(g, &mut ui.text, "M — Carte    I — Inventaire", 640.0, 390.0, 24.0, WHITE, 1);
    txt(g, &mut ui.text, "Q — Quitter la mission", 640.0, 430.0, 24.0, WHITE, 1);
    txt(
        g,
        &mut ui.text,
        "Le combo de mêlée est mémorisé pendant la pause (mécanique MCD).",
        640.0,
        500.0,
        17.0,
        GREY,
        1,
    );
}

fn dead(g: &Gfx, ui: &mut UiFrame, ctx: &Ctx) {
    quad(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, tex::T_WHITE, [0.25, 0.0, 0.0, 0.55]);
    quad(&mut ui.icons, 616.0, 200.0, 48.0, 48.0, tex::T_SKULL, WHITE);
    txt(g, &mut ui.text, "VOUS ÊTES TOMBÉ…", 640.0, 320.0, 48.0, RED, 1);
    txt(
        g,
        &mut ui.text,
        &format!("Vies restantes : {}", ctx.world.lives),
        640.0,
        380.0,
        26.0,
        WHITE,
        1,
    );
    txt(g, &mut ui.text, "ENTRÉE — Réapparaître au point de contrôle", 640.0, 450.0, 22.0, GOLD, 1);
}

fn game_over(g: &Gfx, ui: &mut UiFrame, ctx: &Ctx, t: f32) {
    quad(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, tex::T_WHITE, [0.12, 0.0, 0.0, 0.85]);
    quad(&mut ui.icons, 608.0, 170.0, 64.0, 64.0, tex::T_SKULL, WHITE);
    txt(g, &mut ui.text, "MISSION ÉCHOUÉE", 640.0, 300.0, 54.0, RED, 1);
    stats(g, ui, ctx, 370.0);
    if (t * 1.4) as i32 % 2 == 0 {
        txt(g, &mut ui.text, "ENTRÉE — Recommencer la mission", 640.0, 560.0, 24.0, GOLD, 1);
    }
    txt(g, &mut ui.text, "Q — Quitter", 640.0, 604.0, 20.0, GREY, 1);
}

fn victory(g: &Gfx, ui: &mut UiFrame, ctx: &Ctx, t: f32) {
    quad(&mut ui.icons, 0.0, 0.0, 1280.0, 720.0, tex::T_WHITE, [0.09, 0.07, 0.02, 0.88]);
    txt(g, &mut ui.text, "MISSION RÉUSSIE !", 640.0, 210.0, 58.0, GOLD, 1);
    txt(g, &mut ui.text, "Le Nécromancien est vaincu et les villageois sont sauvés.", 640.0, 270.0, 22.0, WHITE, 1);
    stats(g, ui, ctx, 330.0);
    if (t * 1.4) as i32 % 2 == 0 {
        txt(g, &mut ui.text, "ENTRÉE — Rejouer", 640.0, 580.0, 24.0, GOLD, 1);
    }
}

fn stats(g: &Gfx, ui: &mut UiFrame, ctx: &Ctx, mut y: f32) {
    let s = &ctx.world.stats;
    let p = &ctx.world.player;
    let mins = (s.time / 60.0).floor() as i32;
    let secs = (s.time % 60.0) as i32;
    let rows = [
        ("Temps de mission", format!("{mins:02}:{secs:02}")),
        ("Ennemis vaincus", format!("{}", s.kills)),
        ("Villageois libérés", format!("{}/5", ctx.world.freed)),
        ("Secrets découverts", format!("{}/{}", ctx.world.secrets, crate::game::SECRETS_TOTAL)),
        ("Émeraudes amassées", format!("{}", p.emeralds)),
        ("Niveau atteint", format!("{}", p.level)),
    ];
    for (a, b) in rows {
        txt(g, &mut ui.text, a, 460.0, y, 20.0, GREY, 0);
        txt(g, &mut ui.text, &b, 820.0, y, 20.0, WHITE, 2);
        y += 32.0;
    }
}
