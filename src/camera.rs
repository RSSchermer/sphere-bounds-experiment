use glam::{Mat4, Quat, Vec3};

use crate::optics::Lens;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CameraDescriptor<L> {
    pub lens: L,
    pub position: Vec3,
    pub orientation: Quat,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Camera<L> {
    lens: L,
    position: Vec3,
    orientation: Quat,
}

impl<L> Camera<L>
where
    L: Lens,
{
    pub fn lens(&self) -> &L {
        &self.lens
    }

    pub fn lens_mut(&mut self) -> &mut L {
        &mut self.lens
    }

    pub fn set_lens(&mut self, lens: L) {
        self.lens = lens;
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    pub fn orientation(&self) -> Quat {
        self.orientation
    }

    pub fn set_orientation(&mut self, orientation: Quat) {
        self.orientation = orientation;
    }

    pub fn world_to_camera(&self) -> Mat4 {
        let rotation = Mat4::from_quat(self.orientation);
        let translation = Mat4::from_translation(self.position);
        let camera_to_world = translation * rotation;

        camera_to_world.inverse()
    }

    pub fn world_to_clip(&self) -> Mat4 {
        self.lens.camera_to_clip() * self.world_to_camera()
    }
}

impl<L> From<CameraDescriptor<L>> for Camera<L>
where
    L: Lens,
{
    fn from(descriptor: CameraDescriptor<L>) -> Self {
        let CameraDescriptor {
            lens,
            position,
            orientation,
        } = descriptor;

        Camera {
            lens,
            position,
            orientation,
        }
    }
}

impl<L> From<L> for Camera<L>
where
    L: Lens,
{
    fn from(lens: L) -> Self {
        Camera {
            lens,
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
        }
    }
}
