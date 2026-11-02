// Boîtes instanciées : personnages, accessoires, décors dynamiques.
// Cube unitaire posé sur le sol (0..1 en Y), transformé par instance.
// uv_sel < 0 → couleur pleine ; sinon échantillon de l'atlas sur i_uvr.

struct Globals {
    view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    sun: vec4<f32>,
    fog: vec4<f32>,
    misc: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_smp: sampler;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) i_uvr: vec4<f32>,
    @location(2) color: vec3<f32>,
    @location(3) uv_sel: f32,
    @location(4) wpos: vec3<f32>,
    @location(5) emissive: f32,
};

@vertex
fn vs_main(
    @location(0) c_pos: vec3<f32>,
    @location(1) c_normal: vec3<f32>,
    @location(2) c_uv: vec2<f32>,
    @location(3) c_shade: f32,
    @location(4) i_pos_rot: vec4<f32>,
    @location(5) i_size_uvsel: vec4<f32>,
    @location(6) i_color: vec4<f32>,
    @location(7) i_uvr: vec4<f32>,
    @location(8) i_emissive: f32,
) -> VOut {
    let c = cos(i_pos_rot.w);
    let s = sin(i_pos_rot.w);
    let p = c_pos * i_size_uvsel.xyz;
    let rotated = vec3<f32>(
        p.x * c - p.z * s,
        p.y,
        p.x * s + p.z * c,
    );
    let wpos = rotated + i_pos_rot.xyz + vec3<f32>(0.0, 0.0, 0.0);
    var out: VOut;
    out.pos = g.view_proj * vec4<f32>(wpos, 1.0);
    out.uv = c_uv;
    out.i_uvr = i_uvr;
    out.color = i_color.rgb * c_shade;
    out.uv_sel = i_size_uvsel.w;
    out.wpos = wpos;
    out.emissive = i_emissive;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    var col = in.color;
    if (in.uv_sel >= 0.0) {
        let uv = vec2<f32>(
            mix(in.i_uvr.x, in.i_uvr.z, in.uv.x),
            mix(in.i_uvr.y, in.i_uvr.w, in.uv.y),
        );
        let tex = textureSample(atlas_tex, atlas_smp, uv);
        if (tex.a < 0.5) {
            discard;
        }
        col = tex.rgb * in.color;
    }
    // éclairage : ambiance + soleil doux sur les faces hautes
    let light = g.fog.rgb * 0.85 + vec3<f32>(0.35, 0.33, 0.30) * g.sun.w;
    col = col * light;
    col = mix(col, min(col * 1.6 + vec3<f32>(0.15), vec3<f32>(1.4)), in.emissive);
    let d = distance(in.wpos, g.cam_pos.xyz);
    let f = 1.0 - exp(-pow(d * g.fog.w, 2.0));
    col = mix(col, g.fog.rgb, clamp(f, 0.0, 1.0));
    return vec4<f32>(col, 1.0);
}
