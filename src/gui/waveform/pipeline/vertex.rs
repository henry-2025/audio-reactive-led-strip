use glam::Vec2;
use iced::widget::shader::wgpu;

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: glam::Vec2,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![
        //position
        0 => Float32x3,
    ];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn vertices() -> [Vertex; 4] {
        [
            Vertex {
                position: Vec2::from_array([-1.0, -1.0]),
            },
            Vertex {
                position: Vec2::from_array([-1.0, 1.0]),
            },
            Vertex {
                position: Vec2::from_array([1.0, 1.0]),
            },
            Vertex {
                position: Vec2::from_array([1.0, -1.0]),
            },
        ]
    }

    pub fn indices() -> [u16; 6] {
        [0, 1, 2, 2, 3, 0]
    }
}
