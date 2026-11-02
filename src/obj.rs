/// Chargeur OBJ minimal (format Wavefront) pour les modèles de remplacement des
/// props, via `assets/models/<nom>.obj`. Conventions : +Y vers le haut, origine au
/// sol, unités en mètres, faces triangulées par éventail, indices négatifs non pris
/// en charge. Sans vt/vn, les UV/normales sont déduites.
use glam::{Vec2, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjVertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub uv: [f32; 2],
}

pub struct ObjMesh {
    pub verts: Vec<ObjVertex>,
    pub indices: Vec<u32>,
}

pub fn load_obj(text: &str) -> Option<ObjMesh> {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut uvs: Vec<Vec2> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut verts: Vec<ObjVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut cache: std::collections::HashMap<[u32; 3], u32> = std::collections::HashMap::new();

    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => {
                let x: f32 = it.next()?.parse().ok()?;
                let y: f32 = it.next()?.parse().ok()?;
                let z: f32 = it.next()?.parse().ok()?;
                positions.push(Vec3::new(x, y, z));
            }
            Some("vt") => {
                let u: f32 = it.next()?.parse().ok()?;
                let v: f32 = it.next()?.parse().ok()?;
                uvs.push(Vec2::new(u, 1.0 - v));
            }
            Some("vn") => {
                let x: f32 = it.next()?.parse().ok()?;
                let y: f32 = it.next()?.parse().ok()?;
                let z: f32 = it.next()?.parse().ok()?;
                normals.push(Vec3::new(x, y, z).normalize_or_zero());
            }
            Some("f") => {
                let mut face: Vec<[u32; 3]> = Vec::new();
                for tok in it {
                    let mut parts = tok.split('/');
                    let pi: u32 = parts.next()?.parse().ok()?;
                    let ti: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    let ni: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    face.push([pi, ti, ni]);
                }
                if face.len() < 3 {
                    continue;
                }
                for k in 2..face.len() {
                    let i0 = push_corner(&mut verts, &mut cache, &positions, &uvs, &normals, face[0]);
                    let i1 = push_corner(&mut verts, &mut cache, &positions, &uvs, &normals, face[k - 1]);
                    let i2 = push_corner(&mut verts, &mut cache, &positions, &uvs, &normals, face[k]);
                    indices.extend_from_slice(&[i0, i1, i2]);
                }
            }
            _ => {}
        }
    }

    if verts.is_empty() || indices.is_empty() {
        return None;
    }
    Some(ObjMesh { verts, indices })
}

fn push_corner(
    verts: &mut Vec<ObjVertex>,
    cache: &mut std::collections::HashMap<[u32; 3], u32>,
    positions: &[Vec3],
    uvs: &[Vec2],
    normals: &[Vec3],
    c: [u32; 3],
) -> u32 {
    if let Some(&i) = cache.get(&c) {
        return i;
    }
    let p = positions.get((c[0].max(1) - 1) as usize).copied().unwrap_or(Vec3::ZERO);
    let t = if c[1] == 0 {
        Vec2::new(p.x * 0.25 + 0.5, p.z * 0.25 + 0.5)
    } else {
        uvs.get((c[1].max(1) - 1) as usize).copied().unwrap_or(Vec2::ZERO)
    };
    let n = if c[2] == 0 {
        Vec3::Y
    } else {
        normals.get((c[2].max(1) - 1) as usize).copied().unwrap_or(Vec3::Y)
    };
    let i = verts.len() as u32;
    verts.push(ObjVertex { pos: p.to_array(), nrm: n.to_array(), uv: t.to_array() });
    cache.insert(c, i);
    i
}
