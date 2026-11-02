// Billboards de particules : quads orientés (caméra ou posés au sol) lus depuis un
// storage buffer. Deux modes de fusion : alpha (fumée, ombres) et additif (lueurs).

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

struct Bill {
    pos_size : vec4<f32>,
    color    : vec4<f32>,
    params   : vec4<f32>, // x = mode (0 caméra, 1 sol), y = tuile atlas, z,w = réserve
};

@group(0) @binding(0) var<uniform> u : Uniforms;
@group(0) @binding(1) var<storage, read> inst : array<Bill>;
@group(0) @binding(2) var atlas_tex : texture_2d<f32>;
@group(0) @binding(3) var atlas_smp : sampler;

struct VOut {
    @builtin(position) clip : vec4<f32>,
    @location(0) uv         : vec2<f32>,
    @location(1) color      : vec4<f32>,
    @location(2) fog        : f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vi : u32, @builtin(instance_index) ii : u32) -> VOut {
    let b = inst[ii];
    var corner = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, -0.5), vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, 0.5), vec2<f32>(-0.5, 0.5)
    );
    let c = corner[vi];
    let size = b.pos_size.w;
    var right : vec3<f32>;
    var up : vec3<f32>;
    if (b.params.x < 0.5) {
        right = u.camera_right.xyz;
        up = u.camera_up.xyz;
    } else {
        right = vec3<f32>(1.0, 0.0, 0.0);
        up = vec3<f32>(0.0, 0.0, 1.0);
    }
    let center = b.pos_size.xyz + (right * c.x + up * c.y) * size;
    let tile = b.params.y;
    let inset = 0.02;
    let tu = ((tile % 64.0) + inset + (c.x + 0.5) * (1.0 - 2.0 * inset)) / 64.0;
    let tv = (floor(tile / 64.0) + inset + (0.5 - c.y) * (1.0 - 2.0 * inset)) / 64.0;
    var o : VOut;
    o.clip = u.view_proj * vec4<f32>(center, 1.0);
    o.uv = vec2<f32>(tu, tv);
    o.color = b.color;
    o.fog = distance(center, u.camera_pos.xyz) * u.fog_color.w;
    return o;
}

fn base_color(i : VOut) -> vec4<f32> {
    let tex = textureSample(atlas_tex, atlas_smp, i.uv);
    let alpha = tex.a * i.color.a;
    var col = tex.rgb * i.color.rgb;
    let fog = 1.0 - exp(-i.fog);
    col = mix(col, u.fog_color.rgb, fog * 0.6);
    return vec4<f32>(col * alpha, alpha);
}

@fragment
fn fs_alpha(i : VOut) -> @location(0) vec4<f32> {
    return base_color(i);
}

@fragment
fn fs_add(i : VOut) -> @location(0) vec4<f32> {
    return base_color(i);
}
