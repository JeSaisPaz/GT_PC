use std::num::NonZero;
use wgpu::{
    Device, Queue, TextureFormat, Surface, SurfaceConfiguration,
    StoreOp, VertexStepMode, PrimitiveTopology, FrontFace, Face,
    CompareFunction, ColorWrites, BlendState, PrimitiveState,
    DepthStencilState, DepthBiasState, IndexFormat, BindGroup,
    Buffer, RenderPipeline, PipelineCompilationOptions,
    MipmapFilterMode,
};
use winit::window::Window;
use bytemuck::bytes_of;

use crate::loader::{TrackModel, TrackTexture};

const TRACK_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var track_texture: texture_2d<f32>;
@group(0) @binding(2) var track_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = (uniforms.model * vec4<f32>(input.position, 1.0)).xyz;
    output.clip_pos = uniforms.view_proj * vec4<f32>(world_pos, 1.0);
    output.world_pos = world_pos;
    output.normal = normalize((uniforms.model * vec4<f32>(input.normal, 0.0)).xyz);
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.35;
    let diffuse = max(dot(input.normal, light_dir), 0.0) * 0.65;
    let lighting = ambient + diffuse;

    let tex_color = textureSample(track_texture, track_sampler, input.uv);
    let base_color = tex_color.rgb;

    return vec4<f32>(base_color * lighting, 1.0);
}
"#;

const FALLBACK_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = (uniforms.model * vec4<f32>(input.position, 1.0)).xyz;
    output.clip_pos = uniforms.view_proj * vec4<f32>(world_pos, 1.0);
    output.world_pos = world_pos;
    output.normal = normalize((uniforms.model * vec4<f32>(input.normal, 0.0)).xyz);
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.35;
    let diffuse = max(dot(input.normal, light_dir), 0.0) * 0.65;
    let lighting = ambient + diffuse;

    let grid = step(0.97, fract(input.world_pos.x * 0.05)) +
               step(0.97, fract(input.world_pos.z * 0.05));
    let grid_color = vec3<f32>(0.6) * f32(grid) * 0.2;

    var base_color = vec3<f32>(0.35, 0.38, 0.32);
    base_color = base_color + grid_color;

    return vec4<f32>(base_color * lighting, 1.0);
}
"#;

pub struct Renderer {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    pipeline_tex: RenderPipeline,
    #[allow(dead_code)]
    pipeline_no_tex: RenderPipeline,
    pipeline_lines: RenderPipeline,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    bind_group_lines: BindGroup,
    depth_view: wgpu::TextureView,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
    line_count: u32,
    mesh_index_count: u32,
    has_texture: bool,
}

impl Renderer {
    pub async fn new(window: &'static Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::TEXTURE_COMPRESSION_BC,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                trace: wgpu::Trace::default(),
            },
        ).await.unwrap();

        let format = surface.get_capabilities(&adapter).formats[0];
        let (width, height) = window.inner_size().into();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (_, depth_view) = create_depth_texture(&device, width, height);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (pipeline_tex, pipeline_no_tex, pipeline_lines, bind_group, bind_group_lines) =
            create_resources(&device, &uniform_buffer, format);

        Renderer {
            device,
            queue,
            surface,
            config,
            pipeline_tex,
            pipeline_no_tex,
            pipeline_lines,
            uniform_buffer,
            bind_group,
            bind_group_lines,
            depth_view,
            vertex_buffer,
            index_buffer,
            index_count: 0,
            line_count: 0,
            mesh_index_count: 0,
            has_texture: false,
        }
    }

    pub fn set_texture(&mut self, tex: &TrackTexture) {
        let rgba_data: &[u8] = bytemuck::cast_slice(&tex.data);

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Track Texture"),
            size: wgpu::Extent3d {
                width: tex.width,
                height: tex.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let bytes_per_row = tex.width * 4;

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(tex.height),
            },
            wgpu::Extent3d {
                width: tex.width,
                height: tex.height,
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Track Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        let tex_bgl = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Tex Uniform BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZero::new(128),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tex Uniform Bind Group"),
            layout: &tex_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.bind_group = bind_group;
        self.has_texture = true;
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let (_, dv) = create_depth_texture(&self.device, width, height);
        self.depth_view = dv;
    }

    pub fn render(&mut self, track: &TrackModel, freecam: &crate::camera::Freecam, width: u32, height: u32) {
        if !track.vertices.is_empty() && self.index_count == 0 {
            self.update_buffers(track);
        }

        let view_proj = freecam.get_view_projection_matrix(width as f32 / height as f32);
        let model = glam::Mat4::IDENTITY;

        let mut uniform_data = [0u8; 128];
        uniform_data[0..64].copy_from_slice(bytemuck::bytes_of(&view_proj.to_cols_array()));
        uniform_data[64..128].copy_from_slice(bytemuck::bytes_of(&model.to_cols_array()));
        self.queue.write_buffer(&self.uniform_buffer, 0, &uniform_data);

        let current = self.surface.get_current_texture();
        let st = match current {
            wgpu::CurrentSurfaceTexture::Success(st) => st,
            _ => return,
        };

        let view = st.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Track Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.18, a: 1.0 }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                multiview_mask: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Lines pass (bounding boxes) - always visible
            pass.set_pipeline(&self.pipeline_lines);
            pass.set_bind_group(0, &self.bind_group_lines, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
            if self.line_count > 0 {
                pass.draw_indexed(0..self.line_count, 0, 0..1);
            }

            // Mesh pass (actual triangles) - only if we have mesh data
            if self.mesh_index_count > 0 {
                if self.has_texture {
                    pass.set_pipeline(&self.pipeline_tex);
                    pass.set_bind_group(0, &self.bind_group, &[]);
                } else {
                    pass.set_pipeline(&self.pipeline_no_tex);
                    pass.set_bind_group(0, &self.bind_group_lines, &[]);
                }
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
                pass.draw_indexed(self.line_count..self.line_count + self.mesh_index_count, 0, 0..1);
            }
        }

        self.queue.submit([encoder.finish()]);
        st.present();
    }

    fn update_buffers(&mut self, track: &TrackModel) {
        let vertex_data: Vec<u8> = track.vertices.iter().enumerate().flat_map(|(i, v)| {
            let mut bytes = [0u8; 32];

            bytes[0..4].copy_from_slice(bytes_of(&v.0));
            bytes[4..8].copy_from_slice(bytes_of(&v.1));
            bytes[8..12].copy_from_slice(bytes_of(&v.2));

            let norm = track.normals.get(i).copied().unwrap_or((0.0, 1.0, 0.0));
            bytes[12..16].copy_from_slice(bytes_of(&norm.0));
            bytes[16..20].copy_from_slice(bytes_of(&norm.1));
            bytes[20..24].copy_from_slice(bytes_of(&norm.2));

            let uv = track.uvs.get(i).copied().unwrap_or((0.0, 0.0));
            bytes[24..28].copy_from_slice(bytes_of(&uv.0));
            bytes[28..32].copy_from_slice(bytes_of(&uv.1));
            bytes
        }).collect();

        let index_data: Vec<u32> = track.triangles.iter()
            .flat_map(|(a, b, c)| [*a, *b, *c])
            .collect();

        if vertex_data.is_empty() || index_data.is_empty() {
            return;
        }

        self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: vertex_data.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (index_data.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.queue.write_buffer(&self.vertex_buffer, 0, &vertex_data);
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&index_data));
        self.index_count = index_data.len() as u32;
        self.line_count = (track.line_tri_count * 3) as u32;
        self.mesh_index_count = self.index_count - self.line_count;
    }
}

fn create_resources(
    device: &Device,
    uniform_buffer: &Buffer,
    format: TextureFormat,
) -> (RenderPipeline, RenderPipeline, RenderPipeline, BindGroup, BindGroup) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Track Shader"),
        source: wgpu::ShaderSource::Wgsl(TRACK_SHADER.into()),
    });

    let fallback_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Fallback Shader"),
        source: wgpu::ShaderSource::Wgsl(FALLBACK_SHADER.into()),
    });

    let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Uniform BGL"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: NonZero::new(128),
            },
            count: None,
        }],
    });

    let tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Tex Uniform BGL"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZero::new(128),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let layout_tex = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Textured Layout"),
        bind_group_layouts: &[Some(&tex_bgl)],
        immediate_size: 0,
    });

    let layout_no_tex = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Untextured Layout"),
        bind_group_layouts: &[Some(&uniform_bgl)],
        immediate_size: 0,
    });

    let compilation_options = PipelineCompilationOptions::default();

    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: 32,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 24,
                shader_location: 2,
            },
        ],
    };

    let pipeline_tex = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Textured Pipeline"),
        layout: Some(&layout_tex),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: compilation_options.clone(),
            buffers: &[vertex_layout.clone()],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Cw,
            cull_mode: Some(Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(CompareFunction::Less),
            stencil: Default::default(),
            bias: DepthBiasState::default(),
        }),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: compilation_options.clone(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });

    let pipeline_no_tex = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Untextured Pipeline"),
        layout: Some(&layout_no_tex),
        vertex: wgpu::VertexState {
            module: &fallback_shader,
            entry_point: Some("vs_main"),
            compilation_options: compilation_options.clone(),
            buffers: &[vertex_layout.clone()],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Cw,
            cull_mode: Some(Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(CompareFunction::Less),
            stencil: Default::default(),
            bias: DepthBiasState::default(),
        }),
        fragment: Some(wgpu::FragmentState {
            module: &fallback_shader,
            entry_point: Some("fs_main"),
            compilation_options: compilation_options.clone(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });

    let pipeline_lines = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Line Pipeline"),
        layout: Some(&layout_no_tex),
        vertex: wgpu::VertexState {
            module: &fallback_shader,
            entry_point: Some("vs_main"),
            compilation_options: compilation_options.clone(),
            buffers: &[vertex_layout],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::LineList,
            front_face: FrontFace::Cw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(CompareFunction::Less),
            stencil: Default::default(),
            bias: DepthBiasState::default(),
        }),
        fragment: Some(wgpu::FragmentState {
            module: &fallback_shader,
            entry_point: Some("fs_main"),
            compilation_options,
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });

    let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Uniform BG"),
        layout: &uniform_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let uniform_bg_lines = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Uniform BG Lines"),
        layout: &uniform_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    (pipeline_tex, pipeline_no_tex, pipeline_lines, uniform_bg, uniform_bg_lines)
}

fn create_depth_texture(device: &Device, width: u32, height: u32) -> (TextureFormat, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (TextureFormat::Depth32Float, view)
}
