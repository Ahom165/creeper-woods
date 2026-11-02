// Interface 2D : quads en coordonnées écran (NDC calculé côté CPU), une texture de
// bind group par passe (icônes, police, carte, vignette).

struct VIn {
    @location(0) pos   : vec2<f32>,
    @location(1) uv    : vec2<f32>,
    @location(2) color : vec4<f32>,
};

struct VOut {
    @builtin(position) clip : vec4<f32>,
    @location(0) uv         : vec2<f32>,
    @location(1) color      : vec4<f32>,
};

@group(0) @binding(0) var tex : texture_2d<f32>;
@group(0) @binding(1) var smp : sampler;

@vertex
fn vs_main(v : VIn) -> VOut {
    var o : VOut;
    o.clip = vec4<f32>(v.pos, 0.0, 1.0);
    o.uv = v.uv;
    o.color = v.color;
    return o;
}

@fragment
fn fs_main(i : VOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, smp, i.uv);
    return vec4<f32>(t.rgb * i.color.rgb, t.a * i.color.a);
}
