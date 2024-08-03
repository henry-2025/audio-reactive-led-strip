use iced::widget::shader::wgpu;

// custom buffer type with dynamic resizing
pub struct Buffer {
    label: &'static str,
    pub raw: wgpu::Buffer,
    size: u64,
    usage: wgpu::BufferUsages,
}

impl Buffer {
    pub fn new(
        device: &wgpu::Device,
        label: &'static str,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> Self {
        Self {
            raw: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            }),
            label,
            size,
            usage,
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        new_positions: Vec<f32>,
        new_colors: Vec<Vec<u8>>,
    ) {
        // assert that the new_position and new_colors are the correct length

        // update the current buffers with these new values
    }

    pub fn resize(&mut self, device: &wgpu::Device, new_size: u64) {
        if new_size > self.size {
            self.raw = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: new_size,
                usage: self.usage,
                mapped_at_creation: false,
            });
        }
    }
}
