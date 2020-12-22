use rapier2d::geometry::ColliderHandle;
pub use shrev::*;

#[derive(Debug, Clone)]
pub enum GameEvent {
    Delete(hecs::Entity),

    /// Play the background music.
    PlayBackgroundMusic(String),

    /// Play some sound
    PlaySound(String),

    /// Collision between entities
    ProximityEvent(ColliderHandle, ColliderHandle),
    ContactEvent(ColliderHandle, ColliderHandle),
}
