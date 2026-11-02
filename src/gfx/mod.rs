/// Contexte graphique wgpu : device, pipelines, buffers, passes.
/// Deux passes par image : la scène rendue hors-écran (lumières + brouillard),
/// puis blit vers la surface + interface 2D. Permet la capture PNG headless.
pub mod camera;
pub mod text;
pub mod textures;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt;

use crate::level::Level;
use text::{TextRenderer, UiVertex};

pub const DESIGN_W: f32 = 1280.0;
pub const DESIGN_H: f32 = 720.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub camera_right: [f32; 4],
    pub camera_up: [f32; 4],
    pub sun_dir: [f32; 4],
    pub sun_color: [f32; 4],
    pub ambient: [f32; 4],
    pub fog_color: [f32; 4],
    pub lights_pos: [[f32; 4]; 16],
    pub lights_col: [[f32; 4]; 16],
    pub time: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct InstRaw {
    pub model: [[f32; 4]; 4],
    pub tint: [f32; 4],
    pub uv0: [f32; 2],
    pub uvs: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct BillRaw {
    pub pos_size: [f32; 4],
    pub color: [f32; 4],
    pub params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TerrainVertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub uv: [f32; 2],
    pub emissive: f32,
    pub anim: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CubeVertex {
    pos: [f32; 3],
    nrm: [f32; 3],
    uv: [f32; 2],
}

/// Données d'une image de scène, construites par `render_build`.
pub struct FrameData {
    pub cam: camera::Camera,
    pub chunks: Vec<(u32, u32)>,
    pub insts: Vec<InstRaw>,
    /// Instances des modèles .obj (dessinées après les cubes).
    pub model_insts: Vec<InstRaw>,
    /// (index de modèle, premier indice d'instance, nb d'instances).
    pub model_draws: Vec<(u32, u32, u32)>,
    pub bills: Vec<BillRaw>,
    pub bills_add: Vec<BillRaw>,
    pub lights_pos: [[f32; 4]; 16],
    pub lights_col: [[f32; 4]; 16],
    pub time: [f32; 4],
}

/// Lots d'interface, dessinés dans cet ordre : vignette, carte, icônes, texte.
pub struct UiFrame {
    pub vignette: Vec<UiVertex>,
    pub map: Vec<UiVertex>,
    pub icons: Vec<UiVertex>,
    pub text: Vec<UiVertex>,
}

impl UiFrame {
    pub fn new() -> Self {
        Self { vignette: Vec::new(), map: Vec::new(), icons: Vec::new(), text: Vec::new() }
    }
}

impl Default for UiFrame {
    fn default() -> Self {
        Self::new()
    }
}

const MAX_INST: usize = 8192;
const MAX_BILL: usize = 4096;
const MAX_UI_VERTS: usize = 24_000;

struct ModelRes {
    vb: wgpu::Buffer,
    ib: wgpu::Buffer,
    count: u32,
    bg: wgpu::BindGroup,
}

pub struct Gfx {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pub size: (u32, u32),
    depth_view: wgpu::TextureView,
    offscreen: wgpu::Texture,
    /// Cible composite (scène + interface) : blittée vers la surface et lue par les captures.
    final_tex: wgpu::Texture,
    readback: Option<wgpu::Buffer>,
    models: Vec<ModelRes>,
    bgl_entity: wgpu::BindGroupLayout,

    pub text: TextRenderer,
    font_bg: wgpu::BindGroup,
    map_bg: wgpu::BindGroup,
    vignette_bg: wgpu::BindGroup,
    atlas_ui_bg: wgpu::BindGroup,
    scene_bg: wgpu::BindGroup,
    ent_bg: wgpu::BindGroup,
    bill_a_bg: wgpu::BindGroup,
    bill_g_bg: wgpu::BindGroup,
    blit_bg: wgpu::BindGroup,
    bgl_ui: wgpu::BindGroupLayout,

    p_terrain: wgpu::RenderPipeline,
    p_entity: wgpu::RenderPipeline,
    p_bill_a: wgpu::RenderPipeline,
    p_bill_g: wgpu::RenderPipeline,
    p_ui: wgpu::RenderPipeline,
    p_blit: wgpu::RenderPipeline,

    terrain_vb: Option<wgpu::Buffer>,
    terrain_ib: Option<wgpu::Buffer>,
    cube_vb: wgpu::Buffer,
    cube_ib: wgpu::Buffer,
    inst_sb: wgpu::Buffer,
    bill_a_sb: wgpu::Buffer,
    bill_g_sb: wgpu::Buffer,
    ui_vb: wgpu::Buffer,
    ub: wgpu::Buffer,
}

impl Gfx {
    pub fn new(window: &std::sync::Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .expect("création de la surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("aucun adaptateur Vulkan/DX12/GL disponible");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .expect("création du device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| {
                matches!(f, wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb)
            })
            .copied()
            .unwrap_or(caps.formats[0]);

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let size = (config.width, config.height);

        let depth_view = make_depth(&device, size.0, size.1);
        let offscreen = make_offscreen(&device, size.0, size.1, format);
        let final_tex = make_final(&device, size.0, size.1, format);

        // --- Layouts -------------------------------------------------------------
        let bgl_terrain = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl terrain"),
            entries: &[
                entry_uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                entry_tex(1),
                entry_smp(2),
            ],
        });
        let bgl_entity = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl entite"),
            entries: &[
                entry_uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                entry_tex(2),
                entry_smp(3),
            ],
        });
        let bgl_ui = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl ui"),
            entries: &Self::bgl_ui_entries(),
        });

        // --- Textures ------------------------------------------------------------
        let mut atlas_img = textures::build_atlas();
        textures::apply_overrides(&mut atlas_img, std::path::Path::new("assets/textures"));
        let atlas_view = upload(&device, &queue, &atlas_img)
            .create_view(&wgpu::TextureViewDescriptor::default());

        let smp_near = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let smp_lin = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let atlas_ui_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas ui"),
            layout: &bgl_ui,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&smp_near) },
            ],
        });

        let text = TextRenderer::new(
            include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf").to_vec(),
        );
        let font_bg = ui_bind(&device, &queue, &text.image, &bgl_ui, &smp_lin);
        let vignette_bg = ui_bind(&device, &queue, &textures::build_vignette(), &bgl_ui, &smp_near);
        let map_bg = ui_bind(&device, &queue, &image::RgbaImage::new(4, 4), &bgl_ui, &smp_near);

        // --- Uniforms + storages ---------------------------------------------------
        let ub = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::bytes_of(&Uniforms {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                camera_pos: [0.0; 4],
                camera_right: [0.0; 4],
                camera_up: [0.0; 4],
                sun_dir: [0.0; 4],
                sun_color: [0.0; 4],
                ambient: [0.0; 4],
                fog_color: [0.0; 4],
                lights_pos: [[0.0; 4]; 16],
                lights_col: [[0.0; 4]; 16],
                time: [0.0; 4],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let inst_sb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (MAX_INST * std::mem::size_of::<InstRaw>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bill_a_sb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("billboards alpha"),
            size: (MAX_BILL * std::mem::size_of::<BillRaw>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bill_g_sb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("billboards additifs"),
            size: (MAX_BILL * std::mem::size_of::<BillRaw>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ui_vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("interface"),
            size: (MAX_UI_VERTS * std::mem::size_of::<UiVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let scene_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene"),
            layout: &bgl_terrain,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&smp_near) },
            ],
        });
        let ent_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("entites"),
            layout: &bgl_entity,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: inst_sb.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&smp_near) },
            ],
        });
        let bill_a_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bill_alpha"),
            layout: &bgl_entity,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: bill_a_sb.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&smp_near) },
            ],
        });
        let bill_g_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bill_glow"),
            layout: &bgl_entity,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: bill_g_sb.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&smp_near) },
            ],
        });

        // --- Cube unitaire -----------------------------------------------------------
        let (cube_vb, cube_ib) = make_cube(&device);

        // --- Pipelines ---------------------------------------------------------------
        let m_terrain = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/terrain.wgsl").into()),
        });
        let m_entity = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("entity.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/entity.wgsl").into()),
        });
        let m_bill = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bill.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/bill.wgsl").into()),
        });
        let m_ui = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ui.wgsl").into()),
        });
        let m_blit = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/blit.wgsl").into()),
        });

        let terrain_vbl = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TerrainVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                attr(0, wgpu::VertexFormat::Float32x3, 0),
                attr(1, wgpu::VertexFormat::Float32x3, 12),
                attr(2, wgpu::VertexFormat::Float32x2, 24),
                attr(3, wgpu::VertexFormat::Float32, 32),
                attr(4, wgpu::VertexFormat::Float32, 36),
            ],
        };
        let cube_vbl = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CubeVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                attr(0, wgpu::VertexFormat::Float32x3, 0),
                attr(1, wgpu::VertexFormat::Float32x3, 12),
                attr(2, wgpu::VertexFormat::Float32x2, 24),
            ],
        };
        let ui_vbl = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UiVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                attr(0, wgpu::VertexFormat::Float32x2, 0),
                attr(1, wgpu::VertexFormat::Float32x2, 8),
                attr(2, wgpu::VertexFormat::Float32x4, 16),
            ],
        };

        let layout_of = |bgl: &wgpu::BindGroupLayout, label: &'static str| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[bgl],
                push_constant_ranges: &[],
            })
        };

        let p_terrain = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain"),
            layout: Some(&layout_of(&bgl_terrain, "layout terrain")),
            vertex: wgpu::VertexState {
                module: &m_terrain,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[terrain_vbl],
            },
            fragment: Some(wgpu::FragmentState {
                module: &m_terrain,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: opaque_primitive(),
            depth_stencil: Some(depth_state(true)),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let p_entity = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("entites"),
            layout: Some(&layout_of(&bgl_entity, "layout entites")),
            vertex: wgpu::VertexState {
                module: &m_entity,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[cube_vbl],
            },
            fragment: Some(wgpu::FragmentState {
                module: &m_entity,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: opaque_primitive(),
            depth_stencil: Some(depth_state(true)),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let mk_bill = |entry: &'static str, additive: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("billboard"),
                layout: Some(&layout_of(&bgl_entity, "layout billboard")),
                vertex: wgpu::VertexState {
                    module: &m_bill,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &m_bill,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(if additive {
                            wgpu::BlendState {
                                color: wgpu::BlendComponent::OVER,
                                alpha: wgpu::BlendComponent::OVER,
                            }
                        } else {
                            wgpu::BlendState::ALPHA_BLENDING
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(depth_state(false)),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };
        let p_bill_a = mk_bill("fs_alpha", false);
        let p_bill_g = mk_bill("fs_add", true);

        let p_ui = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("interface"),
            layout: Some(&layout_of(&bgl_ui, "layout interface")),
            vertex: wgpu::VertexState {
                module: &m_ui,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[ui_vbl],
            },
            fragment: Some(wgpu::FragmentState {
                module: &m_ui,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let p_blit = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: Some(&layout_of(&bgl_ui, "layout blit")),
            vertex: wgpu::VertexState {
                module: &m_blit,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &m_blit,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Blit : bind group sur la cible hors-écran.
        let off_view = offscreen.create_view(&wgpu::TextureViewDescriptor::default());
        let blit_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit"),
            layout: &bgl_ui,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&off_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&smp_lin) },
            ],
        });

        Self {
            device,
            queue,
            surface,
            config,
            size,
            depth_view,
            offscreen,
            final_tex,
            readback: None,
            models: Vec::new(),
            bgl_entity,
            text,
            font_bg,
            map_bg,
            vignette_bg,
            atlas_ui_bg,
            scene_bg,
            ent_bg,
            bill_a_bg,
            bill_g_bg,
            blit_bg,
            bgl_ui,
            p_terrain,
            p_entity,
            p_bill_a,
            p_bill_g,
            p_ui,
            p_blit,
            terrain_vb: None,
            terrain_ib: None,
            cube_vb,
            cube_ib,
            inst_sb,
            bill_a_sb,
            bill_g_sb,
            ui_vb,
            ub,
        }
    }

    fn bgl_ui_entries() -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![entry_tex(0), entry_smp(1)]
    }

    pub fn set_models(&mut self, models: &[crate::level::LoadedModel]) {
        self.models.clear();
        let smp = self.device.create_sampler(&wgpu::SamplerDescriptor::default());
        for m in models {
            let vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("modèle vb"),
                contents: bytemuck::cast_slice(&m.mesh.verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ib = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("modèle ib"),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            let img = m.tex.clone().unwrap_or_else(|| {
                image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]))
            });
            let view = upload(&self.device, &self.queue, &img)
                .create_view(&wgpu::TextureViewDescriptor::default());
            let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("modèle"),
                layout: &self.bgl_entity,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.ub.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: self.inst_sb.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&smp) },
                ],
            });
            self.models.push(ModelRes { vb, ib, count: m.mesh.indices.len() as u32, bg });
        }
    }

    pub fn set_terrain(&mut self, level: &Level) {
        self.terrain_vb = Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain vb"),
            contents: bytemuck::cast_slice(&level.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }));
        self.terrain_ib = Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain ib"),
            contents: bytemuck::cast_slice(&level.indices),
            usage: wgpu::BufferUsages::INDEX,
        }));
    }

    pub fn set_map(&mut self, img: &image::RgbaImage) {
        let smp_lin = wgpu::SamplerDescriptor::default();
        self.map_bg = ui_bind(&self.device, &self.queue, img, &self.bgl_ui, &self.device.create_sampler(&smp_lin));
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = make_depth(&self.device, w, h);
        self.offscreen = make_offscreen(&self.device, w, h, self.config.format);
        self.final_tex = make_final(&self.device, w, h, self.config.format);
        let view = self.offscreen.create_view(&wgpu::TextureViewDescriptor::default());
        let smp_lin = self.device.create_sampler(&wgpu::SamplerDescriptor::default());
        self.blit_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit"),
            layout: &self.bgl_ui,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&smp_lin) },
            ],
        });
        self.readback = None;
    }

    /// Bind group blit sur une vue arbitraire (recréé par frame pour final_tex).
    fn make_blit_bind(&self, view: &wgpu::TextureView) -> wgpu::BindGroup {
        let smp = self.device.create_sampler(&wgpu::SamplerDescriptor::default());
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit final"),
            layout: &self.bgl_ui,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&smp) },
            ],
        })
    }

    fn design_map(&self) -> (f32, f32, f32, f32, f32) {
        let w = self.config.width as f32;
        let h = self.config.height as f32;
        let s = (w / DESIGN_W).min(h / DESIGN_H);
        let ox = (w - DESIGN_W * s) * 0.5;
        let oy = (h - DESIGN_H * s) * 0.5;
        (s, ox, oy, w, h)
    }

    /// Convertit un curseur physique en coordonnées de conception 1280×720.
    pub fn cursor_to_design(&self, x: f64, y: f64) -> Vec2 {
        let (s, ox, oy, _, _) = self.design_map();
        Vec2::new(((x as f32) - ox) / s, ((y as f32) - oy) / s)
    }

    /// Convertit un point NDC en coordonnées de conception (dégâts flottants).
    pub fn ndc_to_design(&self, ndc: Vec2) -> Vec2 {
        let (s, ox, oy, w, h) = self.design_map();
        Vec2::new(
            ((ndc.x + 1.0) * 0.5 * w - ox) / s,
            ((1.0 - ndc.y) * 0.5 * h - oy) / s,
        )
    }

    /// Rend une image complète. Renvoie les octets RGBA si `capture`.
    pub fn render(&mut self, frame: &FrameData, ui: &UiFrame, capture: bool) -> Option<Vec<u8>> {
        let n_inst = frame.insts.len().min(MAX_INST);
        let n_bill = frame.bills.len().min(MAX_BILL);
        let n_bill_g = frame.bills_add.len().min(MAX_BILL);

        let (right, up) = frame.cam.right_up();
        let uniforms = Uniforms {
            view_proj: frame.cam.view_proj().to_cols_array_2d(),
            camera_pos: [frame.cam.eye.x, frame.cam.eye.y, frame.cam.eye.z, 1.0],
            camera_right: [right.x, right.y, right.z, 0.0],
            camera_up: [up.x, up.y, up.z, 0.0],
            sun_dir: [-0.35, -1.0, -0.25, 0.55],
            sun_color: [0.45, 0.55, 0.85, 0.0],
            ambient: [0.16, 0.18, 0.26, 0.0],
            fog_color: [0.035, 0.05, 0.115, 0.016],
            lights_pos: frame.lights_pos,
            lights_col: frame.lights_col,
            time: frame.time,
        };
        self.queue.write_buffer(&self.ub, 0, bytemuck::bytes_of(&uniforms));
        if n_inst > 0 {
            self.queue.write_buffer(&self.inst_sb, 0, bytemuck::cast_slice(&frame.insts[..n_inst]));
        }
        if !frame.model_insts.is_empty() {
            self.queue.write_buffer(
                &self.inst_sb,
                (n_inst * std::mem::size_of::<InstRaw>()) as u64,
                bytemuck::cast_slice(&frame.model_insts),
            );
        }
        if n_bill > 0 {
            self.queue.write_buffer(&self.bill_a_sb, 0, bytemuck::cast_slice(&frame.bills[..n_bill]));
        }
        if n_bill_g > 0 {
            self.queue.write_buffer(&self.bill_g_sb, 0, bytemuck::cast_slice(&frame.bills_add[..n_bill_g]));
        }

        let (s, ox, oy, w, h) = self.design_map();
        let mut ui_bytes: Vec<UiVertex> = Vec::new();
        let mut ranges = [(0u32, 0u32); 4];
        for (i, batch) in [&ui.vignette, &ui.map, &ui.icons, &ui.text].into_iter().enumerate() {
            let start = ui_bytes.len();
            for v in batch.iter() {
                let nx = ((v.pos[0] * s + ox) / w) * 2.0 - 1.0;
                let ny = 1.0 - ((v.pos[1] * s + oy) / h) * 2.0;
                ui_bytes.push(UiVertex { pos: [nx, ny], uv: v.uv, color: v.color });
            }
            ranges[i] = (start as u32, (ui_bytes.len() - start) as u32);
        }
        if !ui_bytes.is_empty() {
            self.queue.write_buffer(&self.ui_vb, 0, bytemuck::cast_slice(&ui_bytes));
        }

        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let off_view = self.offscreen.create_view(&wgpu::TextureViewDescriptor::default());
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scène"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &off_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: uniforms.fog_color[0] as f64,
                            g: uniforms.fog_color[1] as f64,
                            b: uniforms.fog_color[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let (Some(vb), Some(ib)) = (&self.terrain_vb, &self.terrain_ib) {
                pass.set_pipeline(&self.p_terrain);
                pass.set_bind_group(0, &self.scene_bg, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                for (first, count) in &frame.chunks {
                    pass.draw_indexed(*first..*first + *count, 0, 0..1);
                }
            }

            if n_inst > 0 {
                pass.set_pipeline(&self.p_entity);
                pass.set_bind_group(0, &self.ent_bg, &[]);
                pass.set_vertex_buffer(0, self.cube_vb.slice(..));
                pass.set_index_buffer(self.cube_ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..36, 0, 0..n_inst as u32);

                // Modèles .obj : même pipeline, autre géométrie + texture.
                if !frame.model_draws.is_empty() {
                    pass.set_vertex_buffer(0, self.cube_vb.slice(..));
                }
                for (mid, first, count) in &frame.model_draws {
                    if let Some(res) = self.models.get(*mid as usize) {
                        let base = n_inst as u32 + *first;
                        pass.set_bind_group(0, &res.bg, &[]);
                        pass.set_vertex_buffer(0, res.vb.slice(..));
                        pass.set_index_buffer(res.ib.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..res.count, 0, base..base + *count);
                    }
                }
            }

            if n_bill > 0 {
                pass.set_pipeline(&self.p_bill_a);
                pass.set_bind_group(0, &self.bill_a_bg, &[]);
                pass.draw(0..6, 0..n_bill as u32);
            }
            if n_bill_g > 0 {
                pass.set_pipeline(&self.p_bill_g);
                pass.set_bind_group(0, &self.bill_g_bg, &[]);
                pass.draw(0..6, 0..n_bill_g as u32);
            }
        }

        if capture {
            let bpr = bytes_per_row(self.config.width);
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("lecture"),
                size: (bpr * self.config.height) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            enc.copy_texture_to_buffer(
                self.offscreen.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bpr),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
            );
            self.readback = Some(buf);
        }

        // Passe composite : blit de la scène + interface 2D sur final_tex.
        {
            let fin_view = self.final_tex.create_view(&wgpu::TextureViewDescriptor::default());
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &fin_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.p_blit);
            pass.set_bind_group(0, &self.blit_bg, &[]);
            pass.draw(0..3, 0..1);

            pass.set_pipeline(&self.p_ui);
            for (i, bg) in [
                &self.vignette_bg,
                &self.map_bg,
                &self.atlas_ui_bg,
                &self.font_bg,
            ]
            .into_iter()
            .enumerate()
            {
                let (start, n) = ranges[i];
                if n > 0 {
                    pass.set_bind_group(0, bg, &[]);
                    pass.set_vertex_buffer(0, self.ui_vb.slice(..));
                    pass.draw(start..start + n, 0..1);
                }
            }
        }
        // Copie de la cible composite pour la capture.
        if capture {
            let bpr = bytes_per_row(self.config.width);
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("lecture"),
                size: (bpr * self.config.height) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            enc.copy_texture_to_buffer(
                self.final_tex.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bpr),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
            );
            self.readback = Some(buf);
        }

        // Présentation : blit de la cible composite vers la surface.
        let surface_tex = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                self.surface.get_current_texture().expect("surface")
            }
        };
        {
            let view = surface_tex
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("présentation"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.p_blit);
            let fin_view = self.final_tex.create_view(&wgpu::TextureViewDescriptor::default());
            pass.set_bind_group(0, &self.make_blit_bind(&fin_view), &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([enc.finish()]);
        surface_tex.present();

        if capture {
            let buf = self.readback.as_ref().expect("readback");
            let bpr = bytes_per_row(self.config.width);
            let slice = buf.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                tx.send(r).ok();
            });
            let _ = self.device.poll(wgpu::Maintain::Wait);
            if rx.recv().is_ok() {
                let data = slice.get_mapped_range();
                let w = self.config.width as usize;
                let h = self.config.height as usize;
                let mut out = Vec::with_capacity(w * h * 4);
                for y in 0..h {
                    for x in 0..w {
                        let o = y * bpr as usize + x * 4;
                        out.push(data[o + 2]);
                        out.push(data[o + 1]);
                        out.push(data[o]);
                        out.push(data[o + 3]);
                    }
                }
                drop(data);
                buf.unmap();
                Some(out)
            } else {
                None
            }
        } else {
            None
        }
    }

}

fn bytes_per_row(w: u32) -> u32 {
    (w * 4).div_ceil(256) * 256
}

fn make_depth(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("profondeur"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn make_final(device: &wgpu::Device, w: u32, h: u32, format: wgpu::TextureFormat) -> wgpu::Texture {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("composite"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
}

fn make_offscreen(
    device: &wgpu::Device,
    w: u32,
    h: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hors-écran"),
        size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn entry_uniform(binding: u32, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn entry_tex(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn entry_smp(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn attr(loc: u32, format: wgpu::VertexFormat, offset: u64) -> wgpu::VertexAttribute {
    wgpu::VertexAttribute { format, offset, shader_location: loc }
}

fn opaque_primitive() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        ..Default::default()
    }
}

fn depth_state(write: bool) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth24Plus,
        depth_write_enabled: write,
        depth_compare: if write {
            wgpu::CompareFunction::Less
        } else {
            wgpu::CompareFunction::LessEqual
        },
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

fn upload(device: &wgpu::Device, queue: &wgpu::Queue, img: &image::RgbaImage) -> wgpu::Texture {
    let (w, h) = img.dimensions();
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("texture"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        img.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    tex
}

fn ui_bind(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::RgbaImage,
    bgl: &wgpu::BindGroupLayout,
    smp: &wgpu::Sampler,
) -> wgpu::BindGroup {
    let view = upload(device, queue, img).create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ui tex"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(smp) },
        ],
    })
}

fn make_cube(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer) {
    let faces: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::Z, Vec3::X, Vec3::Y),
        (-Vec3::Z, -Vec3::X, Vec3::Y),
        (Vec3::X, -Vec3::Z, Vec3::Y),
        (-Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::X, -Vec3::Z),
        (-Vec3::Y, Vec3::X, Vec3::Z),
    ];
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for (n, t, b) in faces {
        let base = verts.len() as u32;
        for (corner, uv) in [
            (-t - b, [0.0f32, 1.0]),
            (t - b, [1.0, 1.0]),
            (t + b, [1.0, 0.0]),
            (-t + b, [0.0, 0.0]),
        ] {
            verts.push(CubeVertex {
                pos: (n * 0.5 + corner * 0.5).to_array(),
                nrm: n.to_array(),
                uv,
            });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("cube vb"),
        contents: bytemuck::cast_slice(&verts),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("cube ib"),
        contents: bytemuck::cast_slice(&idx),
        usage: wgpu::BufferUsages::INDEX,
    });
    (vb, ib)
}
