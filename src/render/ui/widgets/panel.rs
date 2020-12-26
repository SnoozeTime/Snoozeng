use crate::core::colors::RgbaColor;
use crate::core::window::WindowDim;
use crate::geom2::Vector2f;
use crate::render::ui::{Color, Position, Vertex};

/// A flat zone of same color.
pub struct Panel {
    /// Top-left corner
    pub(crate) anchor: Vector2f,
    /// width and height of the panel
    pub(crate) dimensions: Vector2f,
    /// color of the panel
    pub(crate) color: RgbaColor,
}

impl Panel {
    /// Get the vertices to draw the panel.
    pub(crate) fn vertices(&self, window_dim: WindowDim) -> (Vec<Vertex>, Vec<u32>) {
        debug!("WindowDim -> {:#?}", window_dim);

        let w = window_dim.width as f32;
        let h = window_dim.height as f32;
        let x = (self.anchor.x / w) * 2.0 - 1.0;
        let y = (1.0 - self.anchor.y / h) * 2.0 - 1.0;
        let top_left = Vector2f::new(x, y);
        debug!("Anchor -> {:#?}", self.anchor);
        debug!("Top left -> {:#?}", top_left);
        let dim = Vector2f::new(2.0 * self.dimensions.x / w, 2.0 * self.dimensions.y / h);
        debug!("Dimensions = {:?} => {:?}", self.dimensions, dim);
        let top_right = top_left + dim.x * Vector2f::x();
        let bottom_right = top_left + dim.x * Vector2f::x() - dim.y * Vector2f::y();
        let bottom_left = top_left - dim.y * Vector2f::y();

        let color = self.color.to_normalized();
        (
            vec![
                Vertex {
                    position: Position::new(bottom_left.into()),
                    color: Color::new(color),
                },
                Vertex {
                    position: Position::new(top_left.into()),
                    color: Color::new(color),
                },
                Vertex {
                    position: Position::new(top_right.into()),
                    color: Color::new(color),
                },
                Vertex {
                    position: Position::new(bottom_right.into()),
                    color: Color::new(color),
                },
            ],
            vec![0, 1, 2, 0, 2, 3],
        )
    }
}
