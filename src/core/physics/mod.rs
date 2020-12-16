use crate::core::transform::Transform;
use crate::geom2::Vector2f;
use rapier2d::dynamics::{
    BodyStatus, IntegrationParameters, JointSet, RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
};
use rapier2d::geometry::{
    BroadPhase, Collider, ColliderBuilder, ColliderSet, InteractionGroups, NarrowPhase,
};
use rapier2d::pipeline::{PhysicsPipeline, QueryPipeline};
use serde_derive::{Deserialize, Serialize};

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
            handle: None,
            damping: 0.0,
            interaction_group: InteractionGroups::none(),
        }
    }

    pub fn new_dynamic_cuboid(hx: f32, hy: f32) -> Self {
        Self {
            status: BodyStatus::Dynamic,
            collider: ColliderComponent::Aabb(hx, hy),
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
    pub fn to_collider(&self, interaction_groups: InteractionGroups) -> Collider {
        let ColliderComponent::Aabb(hx, hy) = self;
        ColliderBuilder::cuboid(*hx, *hy)
            .collision_groups(interaction_groups)
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
        let mut pipeline = PhysicsPipeline::new();
        let integration_parameters = IntegrationParameters::default();
        let mut broad_phase = BroadPhase::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut joints = JointSet::new();

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
                c.collider.to_collider(c.interaction_group),
                handle,
                &mut self.bodies,
            );
            c.handle = Some(handle);
            handle
        }
    }

    pub fn step(&mut self) {
        let gravity = rapier2d::na::Vector2::new(0.0, self.config.gravity);
        let mut pipeline = &mut self.pipeline;
        let event_handler = ();
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

    pub fn synchronize(&self, world: &hecs::World) {
        for (_, (transform, rbc)) in world
            .query::<(&mut Transform, &RigidBodyComponent)>()
            .iter()
        {
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
}
