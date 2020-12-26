//! Clean entities the right way. Done at the end of a frame.

use crate::event::{CustomGameEvent, EventQueue, GameEvent};
use crate::resources::Resources;
use log::{debug, info};
use shrev::ReaderId;

/// ahahaha what a confusing name.
pub struct GarbageCollector<GE>
where
    GE: CustomGameEvent,
{
    rdr_id: ReaderId<GameEvent<GE>>,
}

impl<GE> GarbageCollector<GE>
where
    GE: CustomGameEvent + 'static,
{
    pub fn new(resources: &mut Resources) -> Self {
        let mut chan = resources.fetch_mut::<EventQueue<GE>>().unwrap();
        let rdr_id = chan.register_reader();
        Self { rdr_id }
    }

    pub fn collect(&mut self, world: &mut hecs::World, resources: &Resources) {
        let chan = resources.fetch::<EventQueue<GE>>().unwrap();
        for ev in chan.read(&mut self.rdr_id) {
            if let GameEvent::Delete(e) = ev {
                log::debug!("Will delete {:?}", e);

                // TODO Remove the rigid body if it has one.

                // remove from world
                if let Err(e) = world.despawn(*e) {
                    info!("Entity was already deleted (or does not exist?) = {}", e);
                } else {
                    debug!("Entity successfully deleted.");
                }
            }
        }
    }
}
