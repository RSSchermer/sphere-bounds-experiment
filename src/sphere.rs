use bytemuck::Zeroable;
use empa::abi;

#[derive(abi::Sized, Clone, Copy, PartialEq, Debug, Zeroable)]
#[repr(C)]
pub struct Sphere {
    pub origin: abi::Vec3<f32>,
    pub radius: f32,
}
