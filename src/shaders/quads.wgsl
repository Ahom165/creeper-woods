// Quadrilatères instanciés : billboards (particules, âmes), ombres au sol,
// flammes, anneaux d'invocation. kind : 0 billboard, 1 plan au sol, 2 sol rotatif.

struct Globals {
    view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    fog: vec4<f32>,
    misc: vec4<f32>, // x temps
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_smp: sampler;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) kind: f32,
    @location(3) wpos: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) i_pos_kind: vec4<f32>,  // xyz, kind
    @location(1) i_size_rot: vec4<f32>,  // largeur, hauteur, rot, alpha
    @location(2) i_color: vec4<f32>,
    @location(3) i_uvr: vec4<f32>,
    @location(4) corner: vec2<f32>,
) -> VOut {
    let kind = i_pos_kind.w;
    let w = i_size_rot.x;
    let h = i_size_rot.y;
    let rot = i_size_rot.z;
    let c = cos(rot);
    let s = sin(rot);
    let local = vec2<f32>(corner.x * c - corner.y * s, corner.x * s + corner.y * c);
    var offset = vec3<f32>(0.0);
    if (kind < 0.5) {
        // billboard face caméra
        offset = g.cam_right.xyz * (local.x * w) + g.cam_up.xyz * (local.y * h);
    } else {
        // plan au sol (XZ), hauteur légèrement au-dessus
        offset = vec3<f32>(local.x * w, 0.0, local.y * h);
    }
    let wpos = i_pos_kind.xyz + offset;
    var out: VOut;
    out.pos = g.view_proj * vec4<f32>(wpos, 1.0);
    out.uv = vec2<f32>(
        mix(i_uvr.x, i_uvr.z, corner.x * 0.5 + 0.5),
        mix(i_uvr.y, i_uvr.w, 0.5 - corner.y * 0.5),
    );
    out.color = vec4<f32>(i_color.rgb, i_color.a * i_size_rot.w);
    out.kind = kind;
    out.wpos = wpos;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let tex = textureSample(atlas_tex, atlas_smp, in.uv);
    var col = tex.rgb * in.color.rgb;
    var alpha = tex.a * in.color.a;
    if (in.kind > 0.5) {
        // ombre au sol : pas de brouillard, alpha modulé
        alpha = tex.a * in.color.a;
    } else {
        let d = distance(in.wpos, g.cam_pos.xyz);
        let f = 1.0 - exp(-pow(d * g.fog.w, 2.0));
        col = mix(col, g.fog.rgb, clamp(f, 0.0, 1.0) * 0.6);
    }
    return vec4<f32>(col, alpha);
}
