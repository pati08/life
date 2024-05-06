use std::iter;

use wgpu::util::DeviceExt;
use winit::{event::*, window::Window};

pub const CIRCLE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

// mod texture;
//

/// A circle that will be rendered to the screen.
///
/// Although the circle generally uses normalized device coordinates, it will
/// adjust for aspect ratio.
#[derive(Debug)]
pub struct Circle {
    /// Where the circle will be drawn on the screen, between 0 and 1, where 1
    /// is the top-left and formatted as x, y. This is the position of the
    /// top-left corner of it's bounding box.
    pub location: [f32; 2],
}

impl Circle {
    fn as_instance(&self, radius: f32) -> Instance {
        let normalized_location = [
            self.location[0] * 2.0 - 1.0,
            -1.0 * (self.location[1] * 2.0 - 1.0),
        ];
        let center = [normalized_location[0], normalized_location[1]];
        Instance {
            offset: normalized_location,
            center,
        }
    }
}

fn circle_vertices(radius: f32) -> [Vertex; 6] {
    [
        Vertex {
            position: [-radius, -radius, 0.0],
        },
        Vertex {
            position: [radius, -radius, 0.0],
        },
        Vertex {
            position: [radius, radius, 0.0],
        },
        Vertex {
            position: [-radius, -radius, 0.0],
        },
        Vertex {
            position: [radius, radius, 0.0],
        },
        Vertex {
            position: [-radius, radius, 0.0],
        },
    ]
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
struct Instance {
    offset: [f32; 2],
    center: [f32; 2],
}

impl Instance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials, we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5, not conflict with them later
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

/// A struct that holds the core of the render state.
struct RenderCore<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

pub struct RenderState<'a> {
    core: RenderCore<'a>,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    // NEW!
    // #[allow(dead_code)]
    // diffuse_texture: texture::Texture,
    // diffuse_bind_group: wgpu::BindGroup,
    window: &'a Window,
    instance_buffer: wgpu::Buffer,
    res_buffer: wgpu::Buffer,
    res_bind_group: wgpu::BindGroup,
    num_vertices: u32,
    circles: Vec<Circle>,
    grid_size: f32,
    #[allow(dead_code)]
    radius_buffer: wgpu::Buffer,
    radius_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    color_buffer: wgpu::Buffer,
    color_bind_group: wgpu::BindGroup,
}

impl<'a> RenderState<'a> {
    /// Create a new RenderState, ready for rendering.
    ///
    /// # Args
    /// window:
    /// A reference to a winit window, to which we will be rendering
    ///
    /// grid_size:
    /// The size of the grid. This is from 0 to 1 * the height of the viewport
    pub async fn new(window: &'a Window, grid_size: f32) -> RenderState<'a> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let res_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Resolution Buffer"),
            contents: bytemuck::cast_slice(&[size.width as f32, size.height as f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let res_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Resolution Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let res_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Resolution Bind Group"),
            layout: &res_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: res_buffer.as_entire_binding(),
            }],
        });

        let radius_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Radius Buffer"),
            contents: bytemuck::cast_slice(&[grid_size]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let radius_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Radius Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let radius_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Radius Bind Group"),
            layout: &radius_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: radius_buffer.as_entire_binding(),
            }],
        });

        let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Color Buffer"),
            contents: bytemuck::cast_slice(&CIRCLE_COLOR),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let color_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Color Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let color_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Color Bind Group"),
            layout: &color_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: color_buffer.as_entire_binding(),
            }],
        });

        let instances: Vec<Instance> = Vec::new();

        // Determine the maximum size of the buffer for the current resolution and
        // grid size.
        let instances_max_size =
            ((size.width as f32 / grid_size) + 2.0) * ((size.height as f32 / grid_size) + 2.0);
        let instances_max_size = instances_max_size.round() as u64;

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            // size: std::mem::size_of::<Instance>() as u64 * 80u64,
            size: std::mem::size_of::<Instance>() as u64 * instances_max_size,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        queue.write_buffer(&instance_buffer, 0, bytemuck::cast_slice(&instances));

        // Loads the shader at runtime. Change this for prod, but it makes shader
        // changes faster.
        let shader_string = std::fs::read_to_string("src/shader.wgsl").unwrap();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_string.into()),
        });

        // let depth_texture =
        //     texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &res_bind_group_layout,
                    &radius_bind_group_layout,
                    &color_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), Instance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        let vertices = circle_vertices(grid_size);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let core = RenderCore {
            surface,
            device,
            queue,
            config,
        };

        Self {
            core,
            size,
            render_pipeline,
            vertex_buffer,
            window,
            instance_buffer,
            res_bind_group,
            res_buffer,
            num_vertices: vertices.len() as u32,
            circles: Vec::new(),
            grid_size,
            radius_buffer,
            radius_bind_group,
            color_buffer,
            color_bind_group,
        }
    }

    /// Update the circles to be rendered.
    ///
    /// Args
    /// - f
    /// A function that takes a mutable reference to `Vec<Circle>` and returns
    /// `Option<Vec<Circle>>`. If it returns `Some(v)`, then the current value
    /// will be replaced by the `v`
    pub fn update_circles<F>(&mut self, f: F)
    where
        F: for<'f> FnOnce<(&'f mut Vec<Circle>,), Output = Option<Vec<Circle>>>,
    {
        let new_circles = f(&mut self.circles);
        if let Some(v) = new_circles {
            self.circles = v;
        }

        let new_instances = self
            .circles
            .iter()
            .map(|c| c.as_instance(self.grid_size))
            .collect::<Vec<_>>();

        self.core.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&new_instances),
        );
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    pub fn change_grid_size(&self, new: f32) {
        if new <= 0.0 {
            return;
        }
        let vertices = circle_vertices(new);
        self.core
            .queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.core.config.width = new_size.width;
        self.core.config.height = new_size.height;
        self.core
            .surface
            .configure(&self.core.device, &self.core.config);

        self.core.queue.write_buffer(
            &self.res_buffer,
            0 as wgpu::BufferAddress,
            bytemuck::cast_slice(&[new_size.width as f32, new_size.height as f32]),
        );
    }

    pub fn reconfigure(&mut self) {
        self.resize(self.size);
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {}

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.core.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.core
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.res_bind_group, &[]);
            render_pass.set_bind_group(1, &self.radius_bind_group, &[]);
            render_pass.set_bind_group(2, &self.color_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            render_pass.draw(0..self.num_vertices, 0..self.circles.len() as _);

            render_pass.set_pipeline(&self.render_pipeline);
        }

        self.core.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
