use crate::core::timer::Timer;
use bitflags::_core::time::Duration;
use rapier2d::geometry::ColliderHandle;
pub use shrev::*;

#[derive(Debug, Clone)]
pub enum GameEvent<GE>
where
    GE: CustomGameEvent,
{
    Delete(hecs::Entity),

    /// Play the background music.
    PlayBackgroundMusic(String),

    /// Play some sound
    PlaySound(String),

    /// Collision between entities
    ProximityEvent(ColliderHandle, ColliderHandle),
    ContactEvent(ColliderHandle, ColliderHandle),

    /// Custom event, varies depending on the game.
    GameEvent(GE),
}

pub trait CustomGameEvent: std::fmt::Debug + Clone + Send + Sync + 'static {}

pub struct EventQueue<GE>
where
    GE: CustomGameEvent,
{
    chan: EventChannel<GameEvent<GE>>,
    deferred_events: Option<Vec<(GameEvent<GE>, Timer)>>,
}

impl<GE> EventQueue<GE>
where
    GE: CustomGameEvent,
{
    pub fn new() -> Self {
        Self {
            chan: EventChannel::new(),
            deferred_events: Some(vec![]),
        }
    }

    /// Drain a vector of events into storage.
    pub fn drain_vec_write(&mut self, events: &mut Vec<GameEvent<GE>>) {
        self.chan.drain_vec_write(events);
    }

    /// Write a single event into storage.
    pub fn single_write(&mut self, event: GameEvent<GE>) {
        self.chan.single_write(event);
    }

    pub fn read(&self, reader_id: &mut ReaderId<GameEvent<GE>>) -> EventIterator<GameEvent<GE>> {
        self.chan.read(reader_id)
    }

    pub fn register_reader(&mut self) -> ReaderId<GameEvent<GE>> {
        self.chan.register_reader()
    }

    pub fn add_deferred_event(&mut self, event: GameEvent<GE>, timer: Timer) {
        self.deferred_events
            .as_mut()
            .expect("EventQueue should have a deferred vec.")
            .push((event, timer));
    }

    pub fn update_deferred(&mut self, dt: Duration) {
        let mut events = self
            .deferred_events
            .take()
            .expect("EventQueue should have a deferred vec.");
        events.iter_mut().for_each(|ev| ev.1.tick(dt));
        let (mut to_send, not_yet) = events.drain(..).partition(|(ev, timer)| timer.finished());
        self.deferred_events = Some(not_yet);

        let mut to_send = to_send.drain(..).map(|(ev, _)| ev).collect();
        self.chan.drain_vec_write(&mut to_send);
    }
}
