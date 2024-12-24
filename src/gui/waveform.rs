mod pipeline;

use glam::Vec3;
use iced::mouse;
use iced::widget::shader::{self, wgpu, Viewport};
use iced::Rectangle;
use ndarray::{Array1, Array2, Axis};
pub use pipeline::point::ColorPoint;
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
    pub points: Vec<ColorPoint>,
    pub mel_points: Vec<ColorPoint>,
    mode: WaveformDisplayMode,
}

impl Waveform {
    pub fn new(mode: WaveformDisplayMode) -> Self {
        let mut scene = Self {
            points: vec![],
            mel_points: vec![],
            mode,
        };
        scene.change_amount(MAX);
        scene
    }

    pub fn update_points(&mut self, new_points: &Array2<u8>) {
        self.points = send_buffer_to_color_points(new_points);
    }

    pub fn update_mel_display(&mut self, mel_values: &Array1<f64>) {
        self.mel_points = mel_values_to_color_points(mel_values);
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
                        Some(ColorPoint::new())
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

fn send_buffer_to_color_points(send_buffer: &Array2<u8>) -> Vec<ColorPoint> {
    send_buffer
        .axis_iter(Axis(0))
        .map(|col| ColorPoint {
            color: Vec3::new(
                col[0] as f32 / 255.0,
                col[1] as f32 / 255.0,
                col[2] as f32 / 255.0,
            ),
        })
        .collect()
}

fn mel_values_to_color_points(mel_values: &Array1<f64>) -> Vec<ColorPoint> {
    mel_values
        .iter()
        .map(|e| ColorPoint {
            color: Vec3::new(*e as f32 / 255.0, 0.0, 0.0),
        })
        .collect()
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
        Primitive::new(&self.points, &self.mel_points, self.mode, bounds)
    }
}

/// A collection of `Cube`s that can be rendered.
#[derive(Debug)]
pub struct Primitive {
    raw_points: Vec<Raw>,
    raw_mel_points: Vec<Raw>,
    display_mode: WaveformDisplayMode,
    uniforms: Uniforms,
}

impl Primitive {
    pub fn new(
        points: &[ColorPoint],
        mel_points: &[ColorPoint],
        display_mode: WaveformDisplayMode,
        bounds: Rectangle<f32>,
    ) -> Self {
        let uniforms = Uniforms {
            width: bounds.width,
            height: bounds.height,
            n_points: points.len() as u32,
            n_mel_points: mel_points.len() as u32,
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
                self.raw_points.len() as u32,
                self.raw_mel_points.len() as u32,
            ));
        }

        let pipeline = storage.get_mut::<Pipeline>().unwrap();

        // Upload data to GPU
        pipeline.update(
            device,
            queue,
            &self.uniforms,
            self.raw_points.len() as u32,
            &self.raw_points,
            self.raw_mel_points.len() as u32,
            &self.raw_mel_points,
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
