//! Provide a macro to create SerializableEntity that can be saved, sent over network and so on...

// fn get_component<T>(world: &hecs::World, e: hecs::Entity) -> Option<T>
// where
//     T: Clone + Send + Sync + 'static,
// {
//     world.get::<T>(e).ok().map(|c| (*c).clone())
// }

#[macro_export]
macro_rules! serialize {
    ($(($name:ident, $component:ty)),+) => {


        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        pub struct SerializedEntity {
            $(
                #[serde(skip_serializing_if = "Option::is_none")]
                #[serde(default)]
                pub $name: Option<$component>
            ),+

        }

        impl SerializedEntity {

            pub fn spawn(&self, world: &mut hecs::World, resources: &Resources) -> hecs::Entity {
                let mut builder = hecs::EntityBuilder::new();

                $(
                    if let Some(ref c) = self.$name {
                        builder.add(c.clone());
                    }
                )+

                let e = world.spawn(builder.build());

                // If there is a physic component, let's register some stuff !
                if let Some(mut physics) = resources.fetch_mut::<CollisionWorld>() {
                    if let Ok(t) = world.get::<Transform>(e) {
                        if let Ok(mut rbc) = world.get_mut::<RigidBodyComponent>(e) {
                            physics.add_body_with_entity(&t.translation, &mut rbc, e);
                        }
                    }
                }

                e
            }

            pub fn spawn_at_pos(&self, world: &mut hecs::World, pos: Vector2f, resources: &Resources) -> hecs::Entity {
                let e = self.spawn(world, resources);

                if let Ok(mut t) = world.get_mut::<Transform>(e) {
                    t.translation = pos;
                }

                e
            }
        }
    };
}
