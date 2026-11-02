/// Boucle applicative winit : fenêtre, entrées (scancodes → AZERTY-proof),
/// machine à états (titre → jeu → pause/mort/échec/victoire), caméra 3/4,
/// mode capture headless (`--capture DIR`) pour les captures d'écran et la CI.
use std::time::Instant;

use glam::{Vec2, Vec3};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::gfx::{camera::Camera, textures, Gfx, UiFrame};
use crate::game::{Input, World};
use crate::render_build;
use crate::ui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Playing,
    Paused,
    Dead,
    GameOver,
    Victory,
}

pub struct App {
    pub screen: Screen,
    pub world: World,
    pub map_open: bool,
    pub inv_open: bool,
    pub app_time: f32,
    pub cursor: Vec2,
    pub gfx: Option<Gfx>,
    pub window: Option<std::sync::Arc<Window>>,
    pub cam: Camera,
    pub keys: [bool; 4], // W A S D (physiques, donc ZQSD en AZERTY)
    pub lmb: bool,
    pub rmb: bool,
    pub press: u32,
    pub rmb_release: bool,
    pub last: Option<Instant>,
    pub should_exit: bool,
    pub capture: Option<Capture>,
}

pub struct Capture {
    pub dir: String,
    pub frame: u32,
}

impl App {
    pub fn new(capture_dir: Option<String>) -> Self {
        App {
            screen: Screen::Title,
            world: World::new(),
            map_open: false,
            inv_open: false,
            app_time: 0.0,
            cursor: Vec2::new(640.0, 360.0),
            gfx: None,
            window: None,
            cam: Camera::new(16.0 / 9.0),
            keys: [false; 4],
            lmb: false,
            rmb: false,
            press: 0,
            rmb_release: false,
            last: None,
            should_exit: false,
            capture: capture_dir.map(|dir| Capture { dir, frame: 0 }),
        }
    }

    fn build_input(&self) -> Input {
        let [w, a, s, d] = self.keys;
        let mv = Vec2::new((d as i32 - a as i32) as f32, (s as i32 - w as i32) as f32);
        let aim_world = self.aim_world();
        Input { mv, aim: aim_world, lmb: self.lmb, rmb: self.rmb, press: self.press, rmb_release: self.rmb_release }
    }

    /// Point visé au sol : lance un rayon depuis le curseur (plan y = 0).
    fn aim_world(&self) -> Vec2 {
        let ndc = Vec2::new(
            self.cursor.x / crate::gfx::DESIGN_W * 2.0 - 1.0,
            1.0 - self.cursor.y / crate::gfx::DESIGN_H * 2.0,
        );
        let (o, d) = self.cam.ground_ray(ndc);
        if d.y.abs() < 0.0001 {
            return Vec2::new(o.x, o.z);
        }
        let t = -o.y / d.y;
        if t < 0.0 {
            return Vec2::new(o.x, o.z);
        }
        let p = o + d * t;
        Vec2::new(p.x, p.z)
    }

    fn update_camera(&mut self, dt: f32) {
        let p = self.world.player.pos;
        let aim = self.aim_world();
        let bias = ((aim - p) * 0.16).clamp_length_max(3.5);
        let target = Vec3::new(p.x + bias.x, 0.0, p.y + bias.y);
        let eye = target + Vec3::new(0.0, 23.0, 13.0);
        let k = (1.0 - (-7.0 * dt).exp()).min(1.0);
        self.cam.target += (target - self.cam.target) * k;
        self.cam.eye += (eye - self.cam.eye) * k;
        // Secousse (explosions) via un léger décalage aléatoire.
        if self.world.shake > 0.0 {
            let s = self.world.shake * 0.25;
            self.cam.eye += Vec3::new(
                (self.world.time * 53.0).sin() * s,
                0.0,
                (self.world.time * 47.0).cos() * s,
            );
        }
        self.cam.aspect = self.gfx.as_ref().map_or(16.0 / 9.0, |g| g.size.0 as f32 / g.size.1 as f32);
    }

    fn frame(&mut self) {
        let dt = self
            .last
            .replace(Instant::now())
            .map_or(1.0 / 60.0, |t| (t.elapsed().as_secs_f32()).clamp(1.0 / 240.0, 1.0 / 20.0));
        self.app_time += dt;

        // Script de capture : injecte les entrées AVANT la construction de Input.
        let cap_frame = self.capture.as_ref().map(|c| c.frame);
        if let Some(cf) = cap_frame {
            self.script_capture(cf);
        }
        let input = self.build_input();

        match self.screen {
            Screen::Title => {
                if input.press & crate::game::B_ENTER != 0 {
                    self.world = World::new();
                    self.screen = Screen::Playing;
                }
            }
            Screen::Playing => {
                if input.press & crate::game::B_ESC != 0 {
                    if self.map_open || self.inv_open {
                        self.map_open = false;
                        self.inv_open = false;
                    } else {
                        // Pause : le combo de mêlée est mémorisé (mechanique MCD).
                        self.screen = Screen::Paused;
                    }
                } else if input.press & crate::game::B_M != 0 {
                    self.map_open = !self.map_open;
                    self.inv_open = false;
                } else if input.press & crate::game::B_I != 0 {
                    self.inv_open = !self.inv_open;
                    self.map_open = false;
                } else if self.map_open || self.inv_open {
                    if self.inv_open {
                        if input.press & crate::game::B_1 != 0 {
                            self.world.buy_enchant(0);
                        }
                        if input.press & crate::game::B_2 != 0 {
                            self.world.buy_enchant(1);
                        }
                        if input.press & crate::game::B_3 != 0 {
                            self.world.buy_enchant(2);
                        }
                    }
                    // Monde figé pendant la carte et l'inventaire.
                } else {
                    self.world.update(&input, dt);
                }
                // Mort en cours.
                if self.world.player_dead_t > 0.0 {
                    self.world.player_dead_t -= dt;
                    if self.world.player_dead_t <= 0.0 {
                        self.screen = if self.world.lives > 0 { Screen::Dead } else { Screen::GameOver };
                    }
                }
                if self.world.escaped {
                    self.screen = Screen::Victory;
                }
            }
            Screen::Paused => {
                if input.press & crate::game::B_ESC != 0 {
                    self.screen = Screen::Playing;
                } else if input.press & crate::game::B_M != 0 {
                    self.screen = Screen::Playing;
                    self.map_open = true;
                } else if input.press & crate::game::B_I != 0 {
                    self.screen = Screen::Playing;
                    self.inv_open = true;
                } else if input.press & crate::game::B_Q != 0 {
                    self.should_exit = true;
                    return;
                }
            }
            Screen::Dead => {
                if input.press & crate::game::B_ENTER != 0 {
                    if !self.world.respawn() {
                        self.screen = Screen::GameOver;
                    } else {
                        self.screen = Screen::Playing;
                    }
                }
            }
            Screen::GameOver => {
                if input.press & crate::game::B_ENTER != 0 {
                    self.world.retry();
                    self.screen = Screen::Playing;
                } else if input.press & crate::game::B_Q != 0 {
                    self.should_exit = true;
                    return;
                }
            }
            Screen::Victory => {
                if input.press & crate::game::B_ENTER != 0 {
                    self.screen = Screen::Title;
                }
            }
        }

        if let Some(cf) = cap_frame {
            self.script_capture(cf);
            if let Some(cap) = &mut self.capture {
                cap.frame += 1;
            }
        }

        self.update_camera(dt);
        let ctx = ui::Ctx {
            screen: self.screen,
            world: &self.world,
            map_open: self.map_open,
            inv_open: self.inv_open,
            app_time: self.app_time,
            cursor: self.cursor,
        };
        let cap_dir = self.capture.as_ref().map(|c| c.dir.clone());
        let Some(gfx) = &mut self.gfx else { return };
        let frame = render_build::build(&self.world, &self.cam);
        let mut ui = UiFrame::new();
        {
            let gref = &*gfx;
            ui::draw(gref, &mut ui, &ctx, &self.cam);
        }

        if let Some(dir) = cap_dir {
            let name = Self::capture_name(cap_frame.expect("capture"));
            let milestone = !name.starts_with("frame_");
            if let Some(bytes) = gfx.render(&frame, &ui, milestone) {
                let path = format!("{dir}/{name}");
                if let Some(img) = image::RgbaImage::from_raw(gfx.size.0, gfx.size.1, bytes) {
                    let _ = img.save(&path);
                }
            }
        } else {
            gfx.render(&frame, &ui, false);
        }

        // Réinitialise les bits d'appui consommés cette frame.
        self.press = 0;
        self.rmb_release = false;
    }

    /// Scénario de capture headless : titre → jeu → carte → inventaire → pause.
    /// Modifie directement les entrées persistantes, consommées par la frame.
    fn script_capture(&mut self, frame: u32) {
        match frame {
            40 => self.press |= crate::game::B_ENTER,
            60..=140 => {
                self.keys = [true, false, false, false];
                if frame.is_multiple_of(24) {
                    self.press |= crate::game::B_SPACE;
                }
            }
            141 => self.keys = [false; 4],
            150 => self.press |= crate::game::B_M,
            170 => self.press |= crate::game::B_M,
            180 => self.press |= crate::game::B_I,
            200 => self.press |= crate::game::B_I,
            210 => self.press |= crate::game::B_ESC,
            230 => self.press |= crate::game::B_ESC,
            250 => self.should_exit = true,
            _ => {}
        }
    }

    fn capture_name(frame: u32) -> String {
        match frame {
            30 => "01_titre.png".to_string(),
            100 => "02_gameplay.png".to_string(),
            120 => "03_combat.png".to_string(),
            160 => "04_carte.png".to_string(),
            190 => "05_inventaire.png".to_string(),
            220 => "06_pause.png".to_string(),
            _ => format!("frame_{frame:04}.png"),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Minecraft Dungeons — Creeper Woods (recréation)")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
        let win = event_loop.create_window(attrs).expect("fenêtre");
        let icon = winit::window::Icon::from_rgba(textures::build_window_icon(), 32, 32)
            .expect("icône");
        win.set_window_icon(Some(icon));
        let win = std::sync::Arc::new(win);
        let mut gfx = Gfx::new(&win);
        gfx.set_terrain(&self.world.level);
        gfx.set_map(&self.world.level.map);
        gfx.set_models(&self.world.level.models);
        self.gfx = Some(gfx);
        self.window = Some(win);
        self.last = Some(Instant::now());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.should_exit {
            event_loop.exit();
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(g) = &mut self.gfx {
                    g.resize(size.width, size.height);
                }
            }
            WindowEvent::Focused(false) => {
                if self.screen == Screen::Playing && !self.map_open && !self.inv_open {
                    self.screen = Screen::Paused;
                }
            }
            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if !pressed && is_synthetic {
                    return;
                }
                let code = match event.physical_key {
                    PhysicalKey::Code(c) => c,
                    _ => return,
                };
                let bit = match code {
                    KeyCode::Escape => crate::game::B_ESC,
                    KeyCode::Enter => crate::game::B_ENTER,
                    KeyCode::KeyM => crate::game::B_M,
                    KeyCode::KeyI => crate::game::B_I,
                    KeyCode::KeyE => crate::game::B_E,
                    KeyCode::Space => crate::game::B_SPACE,
                    KeyCode::Digit1 => crate::game::B_1,
                    KeyCode::Digit2 => crate::game::B_2,
                    KeyCode::Digit3 => crate::game::B_3,
                    KeyCode::KeyF => crate::game::B_F,
                    KeyCode::KeyQ => crate::game::B_Q,
                    _ => 0,
                };
                if pressed && bit != 0 && !event.repeat {
                    self.press |= bit;
                }
                // Interaction F pendant le jeu.
                if pressed && code == KeyCode::KeyF && self.screen == Screen::Playing && !self.map_open && !self.inv_open {
                    self.world.try_interact();
                }
                // Mouvement : scancodes physiques (W=Z sur AZERTY).
                let slot = match code {
                    KeyCode::KeyW => Some(0),
                    KeyCode::KeyA => Some(1),
                    KeyCode::KeyS => Some(2),
                    KeyCode::KeyD => Some(3),
                    _ => None,
                };
                if let Some(i) = slot {
                    self.keys[i] = pressed;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    winit::event::MouseButton::Left => {
                        self.lmb = pressed;
                    }
                    winit::event::MouseButton::Right => {
                        self.rmb = pressed;
                        if !pressed {
                            self.rmb_release = true;
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(g) = &self.gfx {
                    self.cursor = g.cursor_to_design(position.x, position.y);
                }
            }
            WindowEvent::RedrawRequested => {
                self.frame();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.should_exit {
            event_loop.exit();
            return;
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
