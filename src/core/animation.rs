use crate::core::timer::Timer;
use crate::event::{CustomGameEvent, EventQueue, GameEvent};
use crate::render::mesh::{Material, MeshRender};
//use crate::render::sprite::Sprite;
use crate::resources::Resources;
use log::error;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// One animation (in one spreadsheet).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Animation {
    /// Keyframes element are sprite_nb and number of frames to elapse for the current
    /// keyframe.
    pub keyframes: Vec<(usize, usize)>,

    /// in frames
    pub current_index: usize,
    pub elapsed_frame: usize,

    pub frame_duration: Timer,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            keyframes: vec![],
            current_index: 0,
            elapsed_frame: 0,
            frame_duration: Timer::of_seconds(1.0 / 60.0),
        }
    }
}

impl Animation {
    pub fn new(keyframes: Vec<(usize, usize)>, frame_duration: Timer) -> Self {
        Self {
            keyframes,
            current_index: 0,
            elapsed_frame: 0,
            frame_duration,
        }
    }

    pub fn last_frame(&self) -> bool {
        self.keyframes.len() == self.current_index + 1
    }
}

/// All Animations for an entity
/// Control what entity is active with current_animation
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AnimationController {
    /// Animation will cycle through the sprites on its spritesheet
    pub animations: HashMap<String, Animation>,

    /// if set to something, will play the corresponding animation
    pub current_animation: Option<String>,

    #[serde(default)]
    pub delete_on_finished: bool,
}

pub struct AnimationSystem;

impl AnimationSystem {
    pub fn animate<GE>(&mut self, world: &mut hecs::World, dt: Duration, resources: &Resources)
    where
        GE: CustomGameEvent,
    {
        let mut events = vec![];
        for (e, (controller, render)) in world
            .query::<(&mut AnimationController, &mut MeshRender)>()
            .iter()
        {
            if let Material::Sprite {
                ref mut sprite_nb, ..
            } = render.material
            {
                if let Some(ref animation_name) = controller.current_animation {
                    if let Some(ref mut animation) = controller.animations.get_mut(animation_name) {
                        *sprite_nb = animation.keyframes[animation.current_index].0.clone() as u32;

                        animation.frame_duration.tick(dt);
                        // Check if one animation frame has elapsed. If yes, then increase the elapsed frame count
                        if animation.frame_duration.finished() {
                            animation.frame_duration.reset();
                            animation.elapsed_frame += 1;
                        }

                        if animation.elapsed_frame > animation.keyframes[animation.current_index].1
                        {
                            animation.elapsed_frame = 0;

                            if animation.last_frame() && controller.delete_on_finished {
                                events.push(GameEvent::Delete(e));
                            }
                            animation.current_index =
                                (animation.current_index + 1) % animation.keyframes.len();
                        }
                    } else {
                        error!("Cannot find animation with name = {}", animation_name);
                    }
                }
            }
        }

        {
            let mut channel = resources.fetch_mut::<EventQueue<GE>>().unwrap();
            channel.drain_vec_write(&mut events);
        }
    }
}
