#import bevy_pbr::mesh_functions::{get_model_matrix, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings::globals
#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing}
#import bevy_pbr::pbr_bindings::material;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) i_pos_scale: vec4<f32>,
};

fn hash(in: f32) -> f32 {
	let large = i32(in * 371311.);
	return f32(large % 91033) / 91033.;
}

fn rotation_axis_matrix(axis: vec3<f32>, angle: f32) -> mat3x3<f32> {
	let x = axis.x;
	let y = axis.y;
	let z = axis.z;
	let sin_a = sin(angle);
	let cos_a = cos(angle);
	let cos_a_i = 1. - cos_a;
	return mat3x3<f32>(
		cos_a + x*x * cos_a_i, x*y * cos_a_i - z * sin_a, x*z * cos_a_i + y * sin_a,
		y*x * cos_a_i + z * sin_a, cos_a + y*y * cos_a_i, y*z * cos_a_i - x * sin_a,
		z*x * cos_a_i - y * sin_a, z*y * cos_a_i + x * sin_a, cos_a + z*z * cos_a_i,
	);
}

fn rotate_axis(point: vec3<f32>, axis: vec3<f32>, angle: f32) -> vec3<f32> {
	return rotation_axis_matrix(axis, angle) * point;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
	let hashed = hash(vertex.i_pos_scale.x + vertex.i_pos_scale.z);
    var position = rotate_axis(vertex.position, vec3<f32>(0., 1., 0.), radians(360.) * hashed) * vertex.i_pos_scale.w + vertex.i_pos_scale.xyz;
    let lambda = clamp(position.y - vertex.i_pos_scale.y, 0., 1.);
	position.x += sin(globals.time * 1. + position.x * 0.01 + position.z * 0.01) * 0.3 * lambda + sin(globals.time * hash(position.x + position.z)) * 0.3 * lambda;

    var out: VertexOutput;
    out.position = mesh_position_local_to_clip(
        get_model_matrix(0u),
        vec4<f32>(position, 1.0)
    );
	out.world_position = vec4<f32>(position, 1.);
	out.world_normal = vertex.normal;
	let color = vec4<f32>(1., 1., 1., 1.);
	out.color = mix(0.1 * color, 1.2 * color, lambda);
    return out;
}

@fragment
fn fragment(
	in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
	let pbr_input = pbr_input_from_standard_material(in, is_front);
    var color = apply_pbr_lighting(pbr_input);
    color = main_pass_post_lighting_processing(pbr_input, color);
	return color;
}
