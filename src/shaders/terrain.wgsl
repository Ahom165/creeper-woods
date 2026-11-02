// Terrain éclairé : lumière ambiante + lune + 16 lampes ponctuelles, brouillard de nuit,
// tuiles émissives (lave, champignons) et défilement d'UV pour l'eau/la lave.

struct Uniforms {
    view_proj   : mat4x4<f32>,
    camera_pos  : vec4<f32>,
    camera_right: vec4<f32>,
    camera_up   : vec4<f32>,
    sun_dir     : vec4<f32>,
    sun_color   : vec4<f32>,
    ambient     : vec4<f32>,
    fog_color   : vec4<f32>,
    lights_pos  : array<vec4<f32>, 16>,
    lights_col  : array<vec4<f32>, 16>,
    time        : vec4<f32>,
};

@group(0) @binding(0) var<uniform> u : Uniforms;
@group(0) @binding(1) var atlas_tex : texture_2d<f32>;
@group(0) @binding(2) var atlas_smp : sampler;

struct VIn {
    @location(0) pos      : vec3<f32>,
    @location(1) nrm      : vec3<f32>,
    @location(2) uv       : vec2<f32>,
    @location(3) emissive : f32,
    @location(4) anim     : f32,
};

struct VOut {
    @builtin(position) clip     : vec4<f32>,
    @location(0) uv             : vec2<f32>,
    @location(1) lum            : vec3<f32>,
    @location(2) emissive       : f32,
    @location(3) fog            : f32,
    @location(4) anim           : f32,
};

fn lighting(wp : vec3<f32>, n : vec3<f32>) -> vec3<f32> {
    var lum = u.ambient.rgb;
    lum = lum + u.sun_color.rgb * max(dot(n, normalize(u.sun_dir.xyz)), 0.0) * u.sun_dir.w;
    for (var i : u32 = 0u; i < 16u; i = i + 1u) {
        let lp = u.lights_pos[i];
        let d = lp.xyz - wp;
        let dist = length(d);
        let att = max(0.0, 1.0 - dist / max(lp.w, 0.001));
        lum = lum + u.lights_col[i].rgb * (att * att * u.lights_col[i].w);
    }
    return lum;
}

@vertex
fn vs_main(v : VIn) -> VOut {
    var o : VOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.uv = v.uv;
    o.lum = lighting(v.pos, v.nrm);
    o.emissive = v.emissive;
    o.anim = v.anim;
    o.fog = distance(v.pos, u.camera_pos.xyz) * u.fog_color.w;
    return o;
}

@fragment
fn fs_main(i : VOut) -> @location(0) vec4<f32> {
    var uv = i.uv;
    if (i.anim > 0.5) {
        uv = uv + vec2<f32>(u.time.x * 0.021, u.time.x * 0.013);
    }
    let tex = textureSample(atlas_tex, atlas_smp, uv);
    var col = tex.rgb * select(i.lum, vec3<f32>(1.2), i.emissive > 0.5);
    let fog = 1.0 - exp(-i.fog);
    col = mix(col, u.fog_color.rgb, fog);
    return vec4<f32>(col, 1.0);
}
