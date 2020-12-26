use crate::geom2::Vector2f;

#[derive(Debug, Copy, Clone)]
pub struct WindowDim {
    pub width: u32,
    pub height: u32,
}

impl WindowDim {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn to_vec2(&self) -> Vector2f {
        Vector2f::new(self.width as f32, self.height as f32)
    }
}
