use buffer::Buffer;
use iced::{widget::shader::wgpu, Rectangle};
mod buffer;

pub struct Pipeline {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: Buffer,
    num_vertices: u64,
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        n_points: u64,
        size: iced::Size<u32>,
    ) -> Self {
        //vertices of one cube
        let vertex_buffer = Buffer::new(
            device,
            "vertex buffer",
            std::mem::size_of::<Vertex>() as u64,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex buffer"),
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
            size: n_points,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("points shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/line.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("points pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self {
            render_pipeline: pipeline,
            vertex_buffer,
            num_vertices: n_points,
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &Vec<Vertex>) {
        //TODO: see if we can resize vertex buffer if cubes amount changed
        // let new_size = num_points * std::mem::size_of::<Vertex>();
        // self.vertices.size(device, new_size as u64);

        let buf_len = (vertices.len() * std::mem::size_of::<Vertex>()) as u64;

        if self.vertex_buffer.raw.size() != buf_len {
            self.vertex_buffer.resize(device, buf_len);
        }

        //always write new cube data since they are constantly rotating
        queue.write_buffer(
            &self.vertex_buffer.raw,
            0,
            bytemuck::cast_slice(vertices.as_slice()),
        );
    }

    pub fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        clear_color: wgpu::Color,
        viewport: Rectangle<u32>,
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_scissor_rect(viewport.x, viewport.y, viewport.width, viewport.height);
            pass.set_pipeline(&self.render_pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.raw.slice(..));
            pass.draw(0..self.num_vertices as u32, 0..1);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [i32; 3],
}

impl Vertex {
    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Sint32x2];
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}
