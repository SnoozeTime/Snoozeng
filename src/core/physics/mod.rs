use crate::core::transform::Transform;
use crate::event::{CustomGameEvent, EventQueue, GameEvent};
use crate::geom2::Vector2f;
use crate::resources::Resources;
use bitflags::_core::cell::RefCell;
use downcast_rs::__std::sync::Mutex;
use rapier2d::dynamics::{
    BodyStatus, IntegrationParameters, JointSet, RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
};
use rapier2d::geometry::{
    BroadPhase, Collider, ColliderBuilder, ColliderSet, ContactEvent, InteractionGroups,
    NarrowPhase, ProximityEvent,
};
use rapier2d::ncollide::na::Isometry2;
use rapier2d::ncollide::query::Proximity;
use rapier2d::pipeline::{EventHandler, PhysicsPipeline};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

pub struct PhysicConfiguration {
    pub gravity: f32,
}

impl Default for PhysicConfiguration {
    fn default() -> Self {
        Self { gravity: -9.81 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RigidBodyComponent {
    status: BodyStatus,
    collider: ColliderComponent,

    /// If true, the component transform will be sync with its physics component. True by default
    pub should_sync: bool,
    pub sensor: bool,

    #[serde(skip)]
    pub handle: Option<RigidBodyHandle>,
    pub damping: f32,
    pub interaction_group: InteractionGroups,
}

impl RigidBodyComponent {
    pub fn new_static_cuboid(hx: f32, hy: f32) -> Self {
        Self {
            status: BodyStatus::Static,
            collider: ColliderComponent::Aabb(hx, hy),
            should_sync: true,
            sensor: false,
            handle: None,
            damping: 0.0,
            interaction_group: InteractionGroups::none(),
        }
    }

    pub fn new_kinematic_cuboid(hx: f32, hy: f32) -> Self {
        Self {
            status: BodyStatus::Kinematic,
            collider: ColliderComponent::Aabb(hx, hy),
            should_sync: true,
            sensor: false,
            handle: None,
            damping: 0.0,
            interaction_group: InteractionGroups::none(),
        }
    }

    pub fn new_dynamic_cuboid(hx: f32, hy: f32) -> Self {
        Self {
            status: BodyStatus::Dynamic,
            collider: ColliderComponent::Aabb(hx, hy),
            sensor: false,
            should_sync: true,

            handle: None,
            damping: 0.0,
            interaction_group: InteractionGroups::none(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ColliderComponent {
    /// Half-extend
    Aabb(f32, f32),
}

impl ColliderComponent {
    pub fn to_collider(&self, interaction_groups: InteractionGroups, is_sensor: bool) -> Collider {
        let ColliderComponent::Aabb(hx, hy) = self;
        ColliderBuilder::cuboid(*hx, *hy)
            .collision_groups(interaction_groups)
            .sensor(is_sensor)
            .build()
    }
}

pub struct CollisionWorld {
    config: PhysicConfiguration,
    colliders: ColliderSet,
    bodies: RigidBodySet,
    pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    joints: JointSet,
}

impl Default for CollisionWorld {
    fn default() -> Self {
        let pipeline = PhysicsPipeline::new();
        let integration_parameters = IntegrationParameters::default();
        let broad_phase = BroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let joints = JointSet::new();

        Self {
            config: PhysicConfiguration::default(),
            joints,
            broad_phase,
            narrow_phase,
            pipeline,
            integration_parameters,
            colliders: ColliderSet::new(),
            bodies: RigidBodySet::new(),
        }
    }
}

impl CollisionWorld {
    pub fn new(config: PhysicConfiguration) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn add_body(
        &mut self,
        translation: &Vector2f,
        c: &mut RigidBodyComponent,
    ) -> RigidBodyHandle {
        if let Some(h) = c.handle {
            h
        } else {
            let body = RigidBodyBuilder::new(c.status)
                .translation(translation.x, translation.y)
                .mass(1.0, false)
                .linear_damping(c.damping)
                .lock_rotations()
                .build();

            let handle = self.bodies.insert(body);
            self.colliders.insert(
                c.collider.to_collider(c.interaction_group, c.sensor),
                handle,
                &mut self.bodies,
            );
            c.handle = Some(handle);
            handle
        }
    }

    pub fn add_body_with_entity(
        &mut self,
        translation: &Vector2f,
        c: &mut RigidBodyComponent,
        e: hecs::Entity,
    ) -> RigidBodyHandle {
        let h = self.add_body(translation, c);
        if let Some(mut rb) = self.bodies.get_mut(h) {
            rb.user_data = e.to_bits() as u128;
        }
        h
    }

    pub fn step<GE>(&mut self, resources: &Resources)
    where
        GE: CustomGameEvent,
    {
        let gravity = rapier2d::na::Vector2::new(0.0, self.config.gravity);
        let pipeline = &mut self.pipeline;
        let mut channel = resources.fetch_mut::<EventQueue<GE>>().unwrap();

        let mut events = Arc::new(Mutex::new(vec![]));
        {
            let event_handler = VecEventHandler(Arc::clone(&events));
            pipeline.step(
                &gravity,
                &self.integration_parameters,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.joints,
                None,
                None,
                &event_handler,
            );
        }

        {
            let locked = events.lock();
            if let Ok(events) = locked {
                for ev in events.iter() {
                    channel.single_write(ev.clone());
                }
            }
        }
    }

    pub fn synchronize(&self, world: &hecs::World) {
        for (_, (transform, rbc)) in world
            .query::<(&mut Transform, &RigidBodyComponent)>()
            .iter()
        {
            if !rbc.should_sync {
                continue;
            }

            if let Some(h) = rbc.handle {
                if let Some(rigid_body) = self.bodies.get(h) {
                    // Update transform with new coordinates.
                    let pos: [f32; 2] = rigid_body.position().translation.vector.into();
                    transform.translation.x = pos[0];
                    transform.translation.y = pos[1];
                }
            }
        }
    }

    pub fn colliders(&self) -> &ColliderSet {
        &self.colliders
    }

    pub fn rigid_bodies(&self) -> &RigidBodySet {
        &self.bodies
    }

    pub fn rigid_bodies_mut(&mut self) -> &mut RigidBodySet {
        &mut self.bodies
    }

    pub fn apply_impulse(&mut self, h: RigidBodyHandle, impulse: Vector2f) {
        if let Some(rb) = self.bodies.get_mut(h) {
            rb.apply_impulse(impulse, true);
        }
    }

    pub fn set_velocity(&mut self, h: RigidBodyHandle, velocity: Vector2f) {
        if let Some(rb) = self.bodies.get_mut(h) {
            rb.set_linvel(velocity, true);
        }
    }

    pub fn set_position(&mut self, h: RigidBodyHandle, position: &Vector2f) {
        if let Some(rb) = self.bodies.get_mut(h) {
            rb.set_position(Isometry2::translation(position.x, position.y), true);
        }
    }
}

struct VecEventHandler<GE>(Arc<Mutex<Vec<GameEvent<GE>>>>)
where
    GE: CustomGameEvent;

impl<GE> EventHandler for VecEventHandler<GE>
where
    GE: CustomGameEvent,
{
    fn handle_proximity_event(&self, event: ProximityEvent) {
        if let Proximity::Intersecting = event.new_status {
            if let Ok(mut events) = self.0.lock() {
                events.push(GameEvent::ProximityEvent(event.collider1, event.collider2));
            }
        }
    }

    fn handle_contact_event(&self, _event: ContactEvent) {
        println!("CONTACT!!");
        // self.0
        //     .push(GameEvent::ContactEvent(event.collider1, event.collider2));
    }
}
