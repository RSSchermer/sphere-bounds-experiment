use std::ops::{Deref, DerefMut};

use glam::f32::Mat4;

pub trait Lens {
    fn camera_to_clip(&self) -> Mat4;

    fn set_aspect_ratio(&mut self, aspect_ratio: f32);
}

impl<L> Lens for Box<L>
where
    L: Lens + ?Sized,
{
    fn camera_to_clip(&self) -> Mat4 {
        self.deref().camera_to_clip()
    }

    fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.deref_mut().set_aspect_ratio(aspect_ratio)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PerspectiveLens {
    pub fov_vertical: f32,
    pub aspect_ratio: f32,
    pub frustum_near: f32,
    pub frustum_far: f32,
}

impl Lens for PerspectiveLens {
    fn camera_to_clip(&self) -> Mat4 {
        let PerspectiveLens {
            fov_vertical,
            aspect_ratio,
            frustum_near,
            frustum_far,
        } = *self;

        Mat4::perspective_rh(fov_vertical, aspect_ratio, frustum_near, frustum_far)
    }

    fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }
}
