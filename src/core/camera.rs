use crate::geom2::{Matrix4f, Vector2f};
use hecs::World;
use rapier2d::na::{Matrix4, Point3, Vector3, Vector4};

/// Camera to display stuff to the screen. If main is true, then it will be used for the rendering.
/// If multiple main camera, then the first one will be used.
#[derive(Debug)]
pub struct Camera {
    pub main: bool,
    pub position: Vector2f,
}

impl Camera {
    pub fn new() -> Camera {
        Self {
            main: true,
            position: Vector2f::zeros(),
        }
    }

    pub fn to_view(&self) -> Matrix4f {
        Matrix4::look_at_rh(
            &Point3::new(self.position.x, self.position.y, 1.0),
            &Point3::new(self.position.x, self.position.y, 0.0),
            &Vector3::new(0.0, 1.0, 0.0),
        )
    }
}

pub fn get_view_matrix(world: &World) -> Option<Matrix4f> {
    world
        .query::<&Camera>()
        .iter()
        .map(|(_, c)| c.to_view())
        .next()
}

pub fn screen_to_world(
    screen_coords: Vector2f,
    projection_matrix: Matrix4f,
    world: &World,
) -> Vector2f {
    let view = get_view_matrix(world).unwrap();
    let pv = projection_matrix * view;
    let inv = pv.try_inverse().unwrap();
    let mouse_pos_world = inv * Vector4::new(screen_coords.x, screen_coords.y, 0.0, 1.0);
    Vector2f::new(mouse_pos_world.x, mouse_pos_world.y)
}

#[derive(Copy, Clone, Debug)]
pub struct ProjectionMatrix(pub(crate) Matrix4f);

impl ProjectionMatrix {
    pub fn new(w: f32, h: f32) -> Self {
        Self(Matrix4f::new_orthographic(0.0, w, 0.0, h, -1.0, 10.0))
    }

    pub fn resize(&mut self, w: f32, h: f32) {
        self.0 = Matrix4f::new_orthographic(0.0, w, 0.0, h, -1.0, 10.0);
    }
}
