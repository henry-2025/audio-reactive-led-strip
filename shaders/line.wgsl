struct Uniform {
    n_points: u32
}

struct LinePoint {
    @position(0) position: vec2f,
    @position(1) color: vec3i,
    @position(2) line_index: u32,
}

struct Output {
    @builtin(position) position: vec4f,
    color: vec3i,
}

@vertex
fn vs_main(in_vertex: LinePoint) -> vec4<f32> {
    return Output {
        position: in_vertex.
        color: in_vertex.color,
    }
}


@fragment
fn fs_main(in: Output) ->
