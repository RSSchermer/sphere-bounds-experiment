use glam::{Quat, Vec3};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GridDescriptor {
    pub scale: f32,
    pub width: usize,
    pub height: usize,
    pub position: Vec3,
    pub orientation: Quat,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Grid {
    scale: f32,
    width: usize,
    height: usize,
    position: Vec3,
    orientation: Quat,
}

impl Grid {
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height;
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
}

impl From<GridDescriptor> for Grid {
    fn from(descriptor: GridDescriptor) -> Self {
        let GridDescriptor {
            scale,
            width,
            height,
            position,
            orientation,
        } = descriptor;

        Grid {
            scale,
            width,
            height,
            position,
            orientation,
        }
    }
}
