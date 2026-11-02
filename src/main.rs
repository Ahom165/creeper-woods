//! Minecraft Dungeons — Creeper Woods (recréation)
//! Moteur maison Rust + wgpu : rendu instancié, éclairage multi-sources,
//! brouillard de nuit, HUD français, Nécromancien, textures procédurales
//! remplaçables par de vraies textures (assets/textures) et props remplaçables
//! par des modèles .obj (assets/models).
//!
//! Usage :
//! - `creeper_woods`            : fenêtre 1280×720
//! - `creeper_woods --capture D` : captures d'écran headless dans D (tests/CI)

mod app;
mod game;
mod gfx;
mod level;
mod obj;
mod render_build;
mod rng;
mod ui;

/// Journal minimal : les erreurs de validation wgpu passent sinon inaperçues.
struct DbgLogger;

impl log::Log for DbgLogger {
    fn enabled(&self, meta: &log::Metadata) -> bool {
        meta.level() <= log::Level::Warn
    }
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!("[{}] {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}

fn main() {
    if std::env::var("MCD_DEBUG").is_ok() {
        let _ = log::set_logger(&DbgLogger);
        log::set_max_level(log::LevelFilter::Warn);
    }
    let mut args = std::env::args().skip(1);
    let capture = loop {
        match args.next().as_deref() {
            Some("--capture") => break args.next(),
            Some(_) => continue,
            None => break None,
        }
    };
    let event_loop = winit::event_loop::EventLoop::new().expect("boucle d'événements");
    let mut application = app::App::new(capture);
    event_loop.run_app(&mut application).expect("exécution");
}
