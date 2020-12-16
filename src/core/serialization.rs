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

            pub fn spawn(&self, world: &mut hecs::World) -> hecs::Entity {
                let mut builder = hecs::EntityBuilder::new();

                $(
                    if let Some(ref c) = self.$name {
                        builder.add(c.clone());
                    }
                )+

                world.spawn(builder.build())

            }

            pub fn spawn_at_pos(&self, world: &mut hecs::World, pos: Vector2f) -> hecs::Entity {
                let e = self.spawn(world);

                if let Ok(mut t) = world.get_mut::<Transform>(e) {
                    t.translation = pos;
                }

                e
            }
        }
    };
}
