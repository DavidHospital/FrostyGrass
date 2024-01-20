struct Vertex {
	position: vec3<f32>,
	_padding: f32,
}

@group(0) @binding(0)
var<storage, read> storage_in: array<Vertex>;

@group(0) @binding(1)
var<storage, read_write> storage_out: array<Vertex>;

@compute @workgroup_size(1, 1, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
	let index = invocation_id.z * num_workgroups.y + invocation_id.y * num_workgroups.x + invocation_id.x;
	storage_out[index] = Vertex(vec3<f32>(invocation_id), 0.);
}

@compute @workgroup_size(1, 1, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {}
