mod pipeline;

use iced::mouse;
use iced::widget::shader::{self, wgpu, Viewport};
use iced::Rectangle;
pub use pipeline::point::Point;
use pipeline::point::Raw;
use pipeline::uniforms::Uniforms;
use pipeline::Pipeline;

use std::cmp::Ordering;
use std::fmt::Display;
use std::iter;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveformDisplayMode {
    Colors,
    RGBChannels,
}

impl WaveformDisplayMode {
    pub const ALL: [WaveformDisplayMode; 2] = [
        WaveformDisplayMode::Colors,
        WaveformDisplayMode::RGBChannels,
    ];
}

impl Display for WaveformDisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WaveformDisplayMode::RGBChannels => "RGB Individual",
                WaveformDisplayMode::Colors => "Color",
            }
        )
    }
}

pub const MAX: u8 = 255;

#[derive(Clone)]
pub struct Waveform {
    pub points: Vec<Point>,
    mode: WaveformDisplayMode,
}

impl Waveform {
    pub fn new(mode: WaveformDisplayMode) -> Self {
        let mut scene = Self {
            points: vec![],
            mode,
        };
        scene.change_amount(MAX);
        scene
    }

    pub fn update_points(&mut self, new_points: Vec<Point>) {
        self.points = new_points;
    }

    pub fn set_mode(&mut self, mode: WaveformDisplayMode) {
        self.mode = mode;
    }

    pub fn change_amount(&mut self, amount: u8) {
        let curr_points = self.points.len() as u8;

        match amount.cmp(&curr_points) {
            Ordering::Greater => {
                // spawn
                let cubes_2_spawn = (amount - curr_points) as usize;

                let mut cubes = 0;
                self.points.extend(iter::from_fn(|| {
                    if cubes < cubes_2_spawn {
                        cubes += 1;
                        Some(Point::new())
                    } else {
                        None
                    }
                }));
            }
            Ordering::Less => {
                // chop
                let cubes_2_cut = curr_points - amount;
                let new_len = self.points.len() - cubes_2_cut as usize;
                self.points.truncate(new_len);
            }
            Ordering::Equal => {}
        }
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
        Primitive::new(&self.points, self.mode, bounds)
    }
}

/// A collection of `Cube`s that can be rendered.
#[derive(Debug)]
pub struct Primitive {
    raw_points: Vec<Raw>,
    display_mode: WaveformDisplayMode,
    uniforms: Uniforms,
}

impl Primitive {
    pub fn new(
        points: &[Point],
        display_mode: WaveformDisplayMode,
        bounds: Rectangle<f32>,
    ) -> Self {
        let uniforms = Uniforms {
            width: bounds.width,
            height: bounds.height,
            n_points: points.len() as u32,
        };

        Self {
            raw_points: match display_mode {
                WaveformDisplayMode::Colors => points
                    .into_iter()
                    .enumerate()
                    .map(Raw::from_point)
                    .collect(),
                WaveformDisplayMode::RGBChannels => points
                    .into_iter()
                    .enumerate()
                    .map(Raw::from_point_split_channels)
                    .flatten()
                    .collect(),
            },
            display_mode,
            uniforms,
        }
    }
}

impl shader::Primitive for Primitive {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        storage: &mut shader::Storage,
        _bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        if !storage.has::<Pipeline>() {
            storage.store(Pipeline::new(
                device,
                queue,
                format,
                viewport.physical_size(),
                self.raw_points.len() as u32,
            ));
        }

        let pipeline = storage.get_mut::<Pipeline>().unwrap();

        // Upload data to GPU
        pipeline.update(
            device,
            queue,
            viewport.physical_size(),
            &self.uniforms,
            self.raw_points.len() as u32,
            &self.raw_points,
        );
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        storage: &shader::Storage,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        // At this point our pipeline should always be initialized
        let pipeline = storage.get::<Pipeline>().unwrap();

        // Render primitive
        pipeline.render(target, encoder, *clip_bounds, self.display_mode);
    }
}
