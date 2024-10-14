use glam::Vec3;
use iced::widget::shader::wgpu;

/// A single instance of a cube.
#[derive(Debug, Clone)]
pub struct Point {
    pub color: Vec3,
}

impl Default for Point {
    fn default() -> Self {
        Self {
            color: glam::Vec3::new(0.0, 1.0, 0.0),
        }
    }
}

impl Point {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, r: f32, g: f32, b: f32) {
        self.color = Vec3::from_array([r, g, b]);
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
#[repr(C)]
pub struct Raw {
    color: Vec3,
    index: u32,
}

impl Raw {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        //point colors
        1 => Float32x3,
        // point index
        2 => Uint32,
    ];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

impl Raw {
    pub fn from_point(input: (usize, &Point)) -> Self {
        Self {
            color: input.1.color,
            index: input.0 as u32,
        }
    }

    pub fn from_point_split_channels(input: (usize, &Point)) -> [Self; 3] {
        [
            Self {
                color: Vec3::new(input.1.color.x, 0.0, 0.0),
                index: input.0 as u32 * 3,
            },
            Self {
                color: Vec3::new(0.0, input.1.color.y, 0.0),
                index: input.0 as u32 * 3 + 1,
            },
            Self {
                color: Vec3::new(0.0, 0.0, input.1.color.z),
                index: input.0 as u32 * 3 + 2,
            },
        ]
    }
}
