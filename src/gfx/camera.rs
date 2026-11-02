use glam::{Mat4, Vec2, Vec3, Vec4};

/// Caméra 3/4 surélevée, à la Minecraft Dungeons : la vue suit le héros, le curseur
/// décale légèrement le point visé pour garder l'objectif de tir lisible.
#[derive(Clone, Copy)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub fov_y: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            eye: Vec3::new(0.0, 23.0, 13.0),
            target: Vec3::ZERO,
            fov_y: 42.0_f32.to_radians(),
            aspect,
        }
    }

    pub fn view_proj(&self) -> Mat4 {
        // wgpu attend une profondeur dans [0, 1] : perspective_rh (pas _rh_gl).
        Mat4::perspective_rh(self.fov_y, self.aspect, 0.5, 320.0)
            * Mat4::look_at_rh(self.eye, self.target, Vec3::Y)
    }

    /// Droite (origine, direction) passant par un point NDC — utilisée pour viser
    /// au curseur : on intersecte le plan du sol y = 0.
    pub fn ground_ray(&self, ndc: Vec2) -> (Vec3, Vec3) {
        let inv = self.view_proj().inverse();
        let near = inv * Vec4::new(ndc.x, ndc.y, 0.0, 1.0);
        let far = inv * Vec4::new(ndc.x, ndc.y, 1.0, 1.0);
        let o = near.truncate() / near.w;
        let f = far.truncate() / far.w;
        let dir = (f - o).normalize_or_zero();
        (o, dir)
    }

    pub fn project(&self, wp: Vec3) -> Option<Vec2> {
        let c = self.view_proj() * Vec4::new(wp.x, wp.y, wp.z, 1.0);
        if c.w <= 0.001 {
            return None;
        }
        Some(Vec2::new(c.x / c.w, c.y / c.w))
    }

    pub fn right_up(&self) -> (Vec3, Vec3) {
        let fwd = (self.target - self.eye).normalize_or_zero();
        let right = fwd.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(fwd).normalize_or_zero();
        (right, up)
    }
}
