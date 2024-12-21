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
    pub display_points: Vec<Point>,
    pub mel_points: Vec<Point>,
    mode: WaveformDisplayMode,
}

impl Waveform {
    pub fn new(mode: WaveformDisplayMode) -> Self {
        let mut scene = Self {
            display_points: vec![],
            mel_points: vec![],
            mode,
        };
        scene.change_amount(MAX);
        scene
    }

    pub fn update_points(&mut self, new_points: Vec<Point>) {
        self.display_points = new_points;
    }

    pub fn update_mel(&mut self, new_points: Vec<Point>) {
        self.mel_points = new_points;
    }

    pub fn set_mode(&mut self, mode: WaveformDisplayMode) {
        self.mode = mode;
    }

    pub fn change_amount(&mut self, amount: u8) {
        let curr_points = self.display_points.len() as u8;

        match amount.cmp(&curr_points) {
            Ordering::Greater => {
                // spawn
                let cubes_2_spawn = (amount - curr_points) as usize;

                let mut cubes = 0;
                self.display_points.extend(iter::from_fn(|| {
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
                let new_len = self.display_points.len() - cubes_2_cut as usize;
                self.display_points.truncate(new_len);
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
        Primitive::new(&self.display_points, &self.mel_points, self.mode, bounds)
    }
}

/// A collection of `Cube`s that can be rendered.
#[derive(Debug)]
pub struct Primitive {
    raw_display_points: Vec<Raw>,
    raw_mel_points: Vec<Raw>,
    display_mode: WaveformDisplayMode,
    uniforms: Uniforms,
}

impl Primitive {
    pub fn new(
        display_points: &[Point],
        mel_points: &[Point],
        display_mode: WaveformDisplayMode,
        bounds: Rectangle<f32>,
    ) -> Self {
        let uniforms = Uniforms {
            width: bounds.width,
            height: bounds.height,
            n_points: display_points.len() as u32,
            n_mel_points: mel_points.len() as u32,
        };

        Self {
            raw_display_points: match display_mode {
                WaveformDisplayMode::Colors => display_points
                    .into_iter()
                    .enumerate()
                    .map(Raw::from_point)
                    .collect(),
                WaveformDisplayMode::RGBChannels => display_points
                    .into_iter()
                    .enumerate()
                    .map(Raw::from_point_split_channels)
                    .flatten()
                    .collect(),
            },
            raw_mel_points: mel_points
                .into_iter()
                .enumerate()
                .map(Raw::from_point)
                .collect(),
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
        _viewport: &Viewport,
    ) {
        if !storage.has::<Pipeline>() {
            storage.store(Pipeline::new(
                device,
                format,
                self.raw_display_points.len() as u32,
                self.raw_mel_points.len() as u32,
            ));
        }

        let pipeline = storage.get_mut::<Pipeline>().unwrap();

        // Upload data to GPU
        pipeline.update(
            device,
            queue,
            &self.uniforms,
            self.raw_display_points.len() as u32,
            &self.raw_display_points,
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
