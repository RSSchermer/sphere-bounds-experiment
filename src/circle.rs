use bytemuck::Zeroable;
use empa::abi;

#[derive(abi::Sized, Clone, Copy, PartialEq, Debug, Zeroable)]
#[repr(C)]
pub struct Circle {
    pub origin: abi::Vec2<f32>,
    pub radius: f32,
}
