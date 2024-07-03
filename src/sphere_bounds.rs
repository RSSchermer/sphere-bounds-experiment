use bytemuck::Zeroable;
use empa::abi;

#[derive(abi::Sized, Clone, Copy, PartialEq, Debug, Zeroable)]
#[repr(C)]
pub struct SphereBounds {
    pub min: abi::Vec2<f32>,
    pub max: abi::Vec2<f32>,
}
