struct Vertex {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<i32>,
}

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<i32>,
}

@vertex
fn vs_main(in_vertex: Vertex) -> Output {
    var out: Output;
    out.position = vec4<f32>(in_vertex.position.x, in_vertex.position.y / 255.0 + 0.5, 0.0, 1.0);
    out.color = in_vertex.color;
    return out;
}


@fragment
fn fs_main(in: Output) -> @location(0) vec4<f32> {
    return vec4<f32>(vec3<f32>(in.color) / 255.0, 1.0);
}
