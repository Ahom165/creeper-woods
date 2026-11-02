// Entités instanciées (ennemis, joueur, props) : un cube unitaire multiplié par une
// matrice d'instance lue depuis un storage buffer, teinte + sélection de tuile atlas.

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

struct Instance {
    model : mat4x4<f32>,
    tint  : vec4<f32>,
    uv0   : vec2<f32>,
    uvs   : vec2<f32>,
};

@group(0) @binding(0) var<uniform> u : Uniforms;
@group(0) @binding(1) var<storage, read> inst : array<Instance>;
@group(0) @binding(2) var atlas_tex : texture_2d<f32>;
@group(0) @binding(3) var atlas_smp : sampler;

struct VIn {
    @location(0) pos : vec3<f32>,
    @location(1) nrm : vec3<f32>,
    @location(2) uv  : vec2<f32>,
};

struct VOut {
    @builtin(position) clip : vec4<f32>,
    @location(0) uv         : vec2<f32>,
    @location(1) lum        : vec3<f32>,
    @location(2) emissive   : f32,
    @location(3) fog        : f32,
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
fn vs_main(v : VIn, @builtin(instance_index) ii : u32) -> VOut {
    let ins = inst[ii];
    let wp = (ins.model * vec4<f32>(v.pos, 1.0)).xyz;
    let n = normalize((ins.model * vec4<f32>(v.nrm, 0.0)).xyz);
    var o : VOut;
    o.clip = u.view_proj * vec4<f32>(wp, 1.0);
    o.uv = ins.uv0 + v.uv * ins.uvs;
    o.lum = lighting(wp, n);
    o.emissive = ins.tint.w;
    o.fog = distance(wp, u.camera_pos.xyz) * u.fog_color.w;
    return o;
}

@fragment
fn fs_main(i : VOut) -> @location(0) vec4<f32> {
    let tex = textureSample(atlas_tex, atlas_smp, i.uv);
    var col = tex.rgb * select(i.lum, vec3<f32>(1.2), i.emissive > 0.5);
    let fog = 1.0 - exp(-i.fog);
    col = mix(col, u.fog_color.rgb, fog);
    return vec4<f32>(col, 1.0);
}
