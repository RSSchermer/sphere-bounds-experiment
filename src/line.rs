use bytemuck::Zeroable;
use empa::abi;

#[derive(abi::Sized, Clone, Copy, PartialEq, Debug, Zeroable)]
#[repr(C)]
pub struct Line {
    pub start: abi::Vec2<f32>,
    pub end: abi::Vec2<f32>,
}
