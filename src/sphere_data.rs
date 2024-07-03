use empa::buffer;
use empa::buffer::Buffer;
use empa::device::Device;
use empa::type_flag::{O, X};
use glam::{Vec3, Vec3A};
use hexasphere::shapes::IcoSphere;

#[derive(empa::render_pipeline::Vertex, Clone, Copy, Debug)]
pub struct Vertex {
    #[vertex_attribute(location = 0, format = "float32x3")]
    position: [f32; 3],
}

impl From<Vec3A> for Vertex {
    fn from(vec: Vec3A) -> Self {
        let vec = Vec3::from(vec);

        Vertex {
            position: [vec.x, vec.y, vec.z],
        }
    }
}

pub struct SphereData {
    pub vertices: Buffer<[Vertex], buffer::Usages<O, O, O, O, X, O, O, O, O, O>>,
    pub indices: Buffer<[u32], buffer::Usages<O, O, O, O, O, X, O, O, O, O>>,
}

impl SphereData {
    pub fn new(device: &Device, subdivisions: usize) -> Self {
        let sphere = IcoSphere::new(subdivisions, |position| Vertex::from(position));

        let vertices = device.create_buffer(sphere.raw_data(), buffer::Usages::vertex());
        let indices =
            device.create_buffer(sphere.get_all_indices().as_slice(), buffer::Usages::index());

        SphereData { vertices, indices }
    }
}
