// SPDX-License-Identifier: MIT OR Apache-2.0
//! Viewport 3D renderer for the editor.
//!
//! This module provides off-screen rendering for the viewport panel,
//! which can later be replaced with ordoplay_render when available.

use egui_wgpu::wgpu;

/// Simple vertex for 3D rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Camera uniforms for the viewport
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

/// Viewport renderer that renders a 3D scene to a texture
pub struct ViewportRenderer {
    /// Render target texture
    render_texture: wgpu::Texture,
    /// View for the render texture
    render_view: wgpu::TextureView,
    /// Depth texture
    depth_texture: wgpu::Texture,
    /// Depth view
    depth_view: wgpu::TextureView,
    /// Current size
    size: [u32; 2],
    /// Render pipeline
    pipeline: wgpu::RenderPipeline,
    /// Grid vertices
    grid_vertex_buffer: wgpu::Buffer,
    /// Grid vertex count
    grid_vertex_count: u32,
    /// Axis vertices (XYZ gizmo at origin)
    axis_vertex_buffer: wgpu::Buffer,
    /// Axis vertex count
    axis_vertex_count: u32,
    /// Camera uniform buffer
    camera_buffer: wgpu::Buffer,
    /// Camera bind group
    camera_bind_group: wgpu::BindGroup,
    /// egui texture ID for the render result
    egui_texture_id: Option<egui::TextureId>,
}

impl ViewportRenderer {
    /// Create a new viewport renderer
    pub fn new(device: &wgpu::Device, initial_size: [u32; 2]) -> Self {
        let size = [initial_size[0].max(1), initial_size[1].max(1)];

        // Create render target texture
        let (render_texture, render_view) = Self::create_render_texture(device, size);
        let (depth_texture, depth_view) = Self::create_depth_texture(device, size);

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Viewport Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("viewport.wgsl").into()),
        });

        // Camera uniform buffer
        let camera_uniform = CameraUniform {
            view_proj: Self::identity_matrix(),
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Viewport Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Viewport Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create grid vertices
        let (grid_vertices, grid_vertex_count) = Self::create_grid_vertices();
        let grid_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&grid_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create axis vertices
        let (axis_vertices, axis_vertex_count) = Self::create_axis_vertices();
        let axis_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Axis Vertex Buffer"),
            contents: bytemuck::cast_slice(&axis_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            render_texture,
            render_view,
            depth_texture,
            depth_view,
            size,
            pipeline,
            grid_vertex_buffer,
            grid_vertex_count,
            axis_vertex_buffer,
            axis_vertex_count,
            camera_buffer,
            camera_bind_group,
            egui_texture_id: None,
        }
    }

    fn create_render_texture(device: &wgpu::Device, size: [u32; 2]) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport Render Texture"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_depth_texture(device: &wgpu::Device, size: [u32; 2]) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport Depth Texture"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn identity_matrix() -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    fn create_grid_vertices() -> (Vec<Vertex>, u32) {
        let mut vertices = Vec::new();
        let grid_size = 10;
        let grid_spacing = 1.0;
        let grid_color = [0.3, 0.3, 0.3];

        // Create grid lines on XZ plane
        for i in -grid_size..=grid_size {
            let pos = i as f32 * grid_spacing;

            // Lines parallel to X axis
            vertices.push(Vertex {
                position: [-grid_size as f32 * grid_spacing, 0.0, pos],
                color: grid_color,
            });
            vertices.push(Vertex {
                position: [grid_size as f32 * grid_spacing, 0.0, pos],
                color: grid_color,
            });

            // Lines parallel to Z axis
            vertices.push(Vertex {
                position: [pos, 0.0, -grid_size as f32 * grid_spacing],
                color: grid_color,
            });
            vertices.push(Vertex {
                position: [pos, 0.0, grid_size as f32 * grid_spacing],
                color: grid_color,
            });
        }

        let count = vertices.len() as u32;
        (vertices, count)
    }

    fn create_axis_vertices() -> (Vec<Vertex>, u32) {
        let axis_length = 2.0;
        let vertices = vec![
            // X axis (red)
            Vertex { position: [0.0, 0.0, 0.0], color: [1.0, 0.2, 0.2] },
            Vertex { position: [axis_length, 0.0, 0.0], color: [1.0, 0.2, 0.2] },
            // Y axis (green)
            Vertex { position: [0.0, 0.0, 0.0], color: [0.2, 1.0, 0.2] },
            Vertex { position: [0.0, axis_length, 0.0], color: [0.2, 1.0, 0.2] },
            // Z axis (blue)
            Vertex { position: [0.0, 0.0, 0.0], color: [0.2, 0.2, 1.0] },
            Vertex { position: [0.0, 0.0, axis_length], color: [0.2, 0.2, 1.0] },
        ];

        let count = vertices.len() as u32;
        (vertices, count)
    }

    /// Resize the viewport
    pub fn resize(&mut self, device: &wgpu::Device, new_size: [u32; 2]) {
        let new_size = [new_size[0].max(1), new_size[1].max(1)];
        if new_size != self.size {
            self.size = new_size;
            let (render_texture, render_view) = Self::create_render_texture(device, new_size);
            let (depth_texture, depth_view) = Self::create_depth_texture(device, new_size);
            self.render_texture = render_texture;
            self.render_view = render_view;
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;
            // Clear the egui texture ID so it gets re-registered
            self.egui_texture_id = None;
        }
    }

    /// Update camera matrices
    pub fn update_camera(
        &self,
        queue: &wgpu::Queue,
        position: [f32; 3],
        target: [f32; 3],
        up: [f32; 3],
        aspect: f32,
        fov: f32,
        near: f32,
        far: f32,
    ) {
        let view = Self::look_at(position, target, up);
        let proj = Self::perspective(fov, aspect, near, far);
        let view_proj = Self::mat4_mul(&proj, &view);

        let uniform = CameraUniform { view_proj };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
        let f = Self::normalize([
            target[0] - eye[0],
            target[1] - eye[1],
            target[2] - eye[2],
        ]);
        let s = Self::normalize(Self::cross(f, up));
        let u = Self::cross(s, f);

        [
            [s[0], u[0], -f[0], 0.0],
            [s[1], u[1], -f[1], 0.0],
            [s[2], u[2], -f[2], 0.0],
            [
                -Self::dot(s, eye),
                -Self::dot(u, eye),
                Self::dot(f, eye),
                1.0,
            ],
        ]
    }

    fn perspective(fov_y_radians: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
        let f = 1.0 / (fov_y_radians / 2.0).tan();
        [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (far + near) / (near - far), -1.0],
            [0.0, 0.0, (2.0 * far * near) / (near - far), 0.0],
        ]
    }

    fn normalize(v: [f32; 3]) -> [f32; 3] {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len > 0.0 {
            [v[0] / len, v[1] / len, v[2] / len]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn mat4_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
        let mut result = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += a[k][j] * b[i][k];
                }
            }
        }
        result
    }

    /// Render the viewport scene
    #[allow(unsafe_code)] // Workaround for wgpu 23 lifetime issue with RenderPass
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, show_grid: bool) {
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Viewport Encoder"),
        });

        // Same workaround for wgpu 23 'static lifetime issue
        let encoder_ptr = Box::into_raw(Box::new(encoder));

        {
            // SAFETY: encoder_ptr is valid and properly reclaimed after render_pass is dropped
            let encoder_ref: &'static mut wgpu::CommandEncoder = unsafe { &mut *encoder_ptr };

            let mut render_pass = encoder_ref.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Viewport Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.12,
                            b: 0.12,
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

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            // Draw grid
            if show_grid {
                render_pass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
                render_pass.draw(0..self.grid_vertex_count, 0..1);
            }

            // Draw axis
            render_pass.set_vertex_buffer(0, self.axis_vertex_buffer.slice(..));
            render_pass.draw(0..self.axis_vertex_count, 0..1);
        }

        // SAFETY: Reclaim the Box after render_pass is dropped
        let encoder = unsafe { Box::from_raw(encoder_ptr) };
        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Get the render texture view for egui integration
    #[allow(dead_code)]
    pub fn get_texture_view(&self) -> &wgpu::TextureView {
        &self.render_view
    }

    /// Get or create the egui texture ID for this viewport
    pub fn get_egui_texture_id(
        &mut self,
        egui_renderer: &mut egui_wgpu::Renderer,
        device: &wgpu::Device,
    ) -> egui::TextureId {
        if let Some(id) = self.egui_texture_id {
            id
        } else {
            let id = egui_renderer.register_native_texture(
                device,
                &self.render_view,
                wgpu::FilterMode::Linear,
            );
            self.egui_texture_id = Some(id);
            id
        }
    }

    /// Get the current size
    #[allow(dead_code)]
    pub fn size(&self) -> [u32; 2] {
        self.size
    }
}

// Re-export for use
use wgpu::util::DeviceExt as _;
