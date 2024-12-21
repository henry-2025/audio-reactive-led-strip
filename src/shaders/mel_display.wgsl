struct Uniforms {
    width: f32,
    height: f32,
    n_mel_points: u32
}

struct Vertex {
    @location(0) position: vec2<f32>
}

struct Point {
    @location(1) color: vec3<f32>,
    @location(2) index: u32,
}

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}


// do all constants in clip space
const MAX_POINTS: f32 = 255.0;
const POINT_WIDTH: f32 = 2.0 / MAX_POINTS;
const POINT_HEIGHT: f32 = POINT_WIDTH * 2.0;

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(vertex: Vertex, point: Point) -> Output {
    var out: Output;

    // want this to stay constant across vertical scaling
    let point_height: f32 = POINT_HEIGHT * uniforms.width / uniforms.height;
    let point_start: f32 = POINT_WIDTH * -f32(uniforms.n_mel_points) / 2.0;

    let point_center: vec2<f32> = vec2<f32>(point_start + POINT_WIDTH / 2 + POINT_WIDTH * f32(point.index), 0.2);

    out.position = vec4<f32>(point_center.x + POINT_WIDTH / 2.0 * vertex.position.x,
                                point_center.y + point_height / 2.0 * vertex.position.y,
                                1.0, 1.0);
    out.color = vec4<f32>(1.0);
    return out;
}

@fragment
fn fs_main(in: Output) -> @location(0) vec4<f32> {
    return in.color;
}
