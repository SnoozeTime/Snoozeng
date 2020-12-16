#[derive(Debug, Clone)]
pub enum GameEvent {
    Delete(hecs::Entity),

    /// Play the background music.
    PlayBackgroundMusic(String),

    /// Play some sound
    PlaySound(String),
}
