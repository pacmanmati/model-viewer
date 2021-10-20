use std::{borrow::Cow, mem};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::window::Window;

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
pub struct Renderer {
    pub instance: Instance,
    pub surface: Surface,
    pub surface_config: SurfaceConfiguration,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub active_pipeline: Option<RenderPipeline>,
    pub vertex_buffer: Option<Buffer>,
    pub index_buffer: Option<Buffer>,
    pub index_count: usize,
    pub bind_group: Option<BindGroup>,
}

/*
 * A wgpu renderer that supports drawing custom models with custom shaders / materials.
 * Axis: camera looks in -Z, +Y is up and +X is right.
 *
 *        ^ +Y
 *        |
 *        |
 *        |
 *        +----------> X+
 *       /
 *     /
 *  |/_ Z+
 *
 */

impl Renderer {
    pub fn new(window: &Window) -> Self {
        let instance = Instance::new(Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .unwrap();
        let window_size = window.inner_size();
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: window_size.width,
            height: window_size.height,
            present_mode: PresentMode::Fifo,
        };
        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: Some("Device"),
                features: Features::default(),
                limits: Limits::default(),
            },
            None,
        ))
        .unwrap();
        surface.configure(&device, &surface_config);

        Self {
            instance,
            surface,
            surface_config,
            adapter,
            device,
            queue,
            active_pipeline: None,
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            bind_group: None,
        }
    }

    pub fn init_cube(&mut self) {
        let cube_positions: &[f32] = &[
            /*
             *        +-[v6]----------+ <-- [v7]
             *       /              / |
             *      /              /  |
             *     /              /   |
             *    /    [v2]      /    |
             *   +-[v4]--------+ [v5] + <-- [v3]
             *   |             |     /
             *   |             |    /
             *   |             |   /
             *   |             |  /
             *   +-[v0]--------+ <-- [v1]
             */
            // -- bottom half
            -0.5, -0.5, -0.5, // v0
            0.5, -0.5, -0.5, // v1
            -0.5, -0.5, 0.5, // v2
            0.5, -0.5, 0.5, // v3
            // -- top half
            -0.5, 0.5, -0.5, // v4
            0.5, 0.5, -0.5, // v5
            -0.5, 0.5, 0.5, // v6
            0.5, 0.5, 0.5, // v7
        ];
        let index_data = &[
            4, 5, 1, 4, 1, 0, 5, 7, 3, 5, 3, 1, 7, 6, 2, 7, 2, 3, 6, 4, 0, 6, 0, 2, 6, 7, 5, 6, 5,
            4, 0, 1, 3, 0, 3, 4,
        ];
        let vertex_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(cube_positions),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(index_data),
            usage: BufferUsages::INDEX,
        });

        let bind_group_layout = self
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Vertex Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(64),
                    },
                    count: None,
                }],
            });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let mx_total = Renderer::generate_matrix(
            self.surface_config.width as f32 / self.surface_config.height as f32,
        );
        let mx_ref: &[f32; 16] = mx_total.as_ref();
        let uniform_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(mx_ref),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let shader = self.device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shader.wgsl"))),
        });
        let vertex_size = mem::size_of::<[f32; 3]>();
        let vertex_buffers = [VertexBufferLayout {
            array_stride: vertex_size as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        }];
        let pipeline = self
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &vertex_buffers,
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    strip_index_format: Some(IndexFormat::Uint32),
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    clamp_depth: false,
                    polygon_mode: PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[self.surface_config.format.into()],
                }),
            });
        self.active_pipeline = Some(pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.index_count = index_data.len();
        self.bind_group = Some(bind_group);
    }

    pub fn draw_cube(renderer: &Renderer) {
        // render here
        let frame = match renderer.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                renderer
                    .surface
                    .configure(&renderer.device, &renderer.surface_config);
                renderer
                    .surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture.")
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.6,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.push_debug_group("preparing data for drawing...");
            rpass.set_pipeline(renderer.active_pipeline.as_ref().unwrap());
            rpass.set_bind_group(0, renderer.bind_group.as_ref().unwrap(), &[]);
            rpass.set_index_buffer(
                renderer.index_buffer.as_ref().unwrap().slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rpass.set_vertex_buffer(0, renderer.vertex_buffer.as_ref().unwrap().slice(..));
            rpass.pop_debug_group();
            rpass.insert_debug_marker("drawing");
            rpass.draw_indexed(0..renderer.index_count as u32, 0, 0..1);
        }
        renderer.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    fn generate_matrix(aspect_ratio: f32) -> cgmath::Matrix4<f32> {
        let mx_projection = cgmath::perspective(cgmath::Deg(45f32), aspect_ratio, 1.0, 10.0);
        let mx_view = cgmath::Matrix4::look_at_rh(
            cgmath::Point3::new(1.5f32, -5.0, 3.0),
            cgmath::Point3::new(0f32, 0.0, 0.0),
            cgmath::Vector3::unit_z(),
        );
        let mx_correction = OPENGL_TO_WGPU_MATRIX;
        mx_correction * mx_projection * mx_view
    }
}
