#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Uniforms {
    pub width: f32,
    pub height: f32,
    pub n_points: u32,
}
