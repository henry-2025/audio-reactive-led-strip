pub mod pipeline;

use iced::{
    mouse,
    widget::shader::{self, wgpu::Color},
    Rectangle,
};
use pipeline::{Pipeline, Vertex};

use crate::config::Config;

#[derive(Clone)]
pub struct Waveform {
    pub size: f32,
    pub vertices: Vec<Vertex>,
    pub background_color: Color,
}

impl Waveform {
    pub fn new() -> Self {
        let config = Config::default();

        let mut scene = Self {
            size: 0.2,
            vertices: vec![],
            background_color: Color::GREEN,
        };

        scene.resize(config.n_points as usize);

        scene
    }

    pub fn update(&mut self, vertices: Vec<Vertex>) {
        assert_eq!(vertices.len(), self.vertices.len());
        self.vertices = vertices;
    }

    pub fn resize(&mut self, new_size: usize) {
        self.vertices.resize(
            new_size,
            Vertex {
                position: [0.0, 0.0],
                color: [0, 0, 0],
            },
        );
    }
}

impl<Message> shader::Program<Message> for Waveform {
    type State = ();
    type Primitive = Primitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        Primitive::new(&self.vertices, self.background_color)
    }
}

/// A collection of `Cube`s that can be rendered.
#[derive(Debug)]
pub struct Primitive {
    vertices: Vec<Vertex>,
    background_color: shader::wgpu::Color,
}

impl Primitive {
    pub fn new(vertices: &Vec<Vertex>, background_color: shader::wgpu::Color) -> Self {
        Self {
            vertices: vertices.clone(),
            background_color,
        }
    }
}

impl shader::Primitive for Primitive {
    fn prepare(
        &self,
        format: shader::wgpu::TextureFormat,
        device: &shader::wgpu::Device,
        queue: &shader::wgpu::Queue,
        bounds: Rectangle,
        target_size: iced::Size<u32>,
        scale_factor: f32,
        storage: &mut shader::Storage,
    ) {
        if !storage.has::<Pipeline>() {
            storage.store(Pipeline::new(
                device,
                queue,
                format,
                self.vertices.len() as u64,
                target_size,
            ));
        }

        let pipeline = storage.get_mut::<Pipeline>().unwrap();

        // Upload data to GPU
        pipeline.update(device, queue, &self.vertices);
    }

    fn render(
        &self,
        storage: &shader::Storage,
        target: &shader::wgpu::TextureView,
        target_size: iced::Size<u32>,
        viewport: Rectangle<u32>,
        encoder: &mut shader::wgpu::CommandEncoder,
    ) {
        // At this point our pipeline should always be initialized
        let pipeline = storage.get::<Pipeline>().unwrap();

        // Render primitive
        pipeline.render(target, encoder, self.background_color, viewport);
    }
}
