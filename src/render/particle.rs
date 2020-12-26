use crate::assets::sprite::SpriteAsset;
use crate::assets::{AssetManager, Handle};
use crate::core::colors::RgbaColor;
use crate::core::curve::Curve;
use crate::core::transform::Transform;
use crate::event::{CustomGameEvent, EventQueue, GameEvent};
use crate::resources::Resources;
use hecs::World;
use luminance::blending::{Blending, Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::pipeline::{PipelineError, TextureBinding};
use luminance::pixel::NormUnsigned;
use luminance::render_state::RenderState;
use luminance::shader::Uniform;

use crate::core::colors;
use crate::geom2::{Matrix4f, Vector2f};
use luminance::tess::Mode;
use luminance::texture::Dim2;
use luminance_derive::UniformInterface;
use luminance_front::tess::Tess;
use luminance_front::{pipeline::Pipeline, shader::Program, shading_gate::ShadingGate};
use rand::Rng;
use rapier2d::na::{Rotation2, Vector3};
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ParticleScale {
    Constant(Vector2f),
    Random(Vector2f, Vector2f),
}

#[derive(Debug, Clone, Default)]
struct Particle {
    life: u32,
    initial_life: u32,
    position: Vector2f,
    velocity: Vector2f,
    colors: Curve<RgbaColor>,
    scale: Vector2f,
    scale_over_lifetime: Option<Curve<f32>>,
    damping: f32,
    rotation: f32,
}

impl Particle {
    fn respawn(
        &mut self,
        life: u32,
        origin: Vector2f,
        velocity: Vector2f,
        scale: Vector2f,
        damping: f32,
        scale_over_lifetime: Option<Curve<f32>>,
        rotation: f32,
    ) {
        self.life = life;
        self.position = origin;
        self.scale = scale;
        self.velocity = velocity;
        self.damping = damping;
        self.scale_over_lifetime = scale_over_lifetime;
        self.initial_life = life;
        self.rotation = rotation;
    }

    /// return true if the particle is still alive
    fn alive(&self) -> bool {
        self.life > 0
    }

    fn update(&mut self, dt: f32) {
        self.velocity *= (1.0 - self.damping / 1000.0);
        self.position += self.velocity.clone() * dt;
        self.life -= 1; // one frame.
    }

    fn t(&self) -> f32 {
        1.0 - self.life as f32 / self.initial_life as f32
    }

    fn color(&self) -> RgbaColor {
        let t = self.t();
        //  println!("{} -> {:?}", t, self.colors.y(t));
        self.colors.y(t)
    }

    fn scale(&self) -> Vector2f {
        if let Some(curve) = &self.scale_over_lifetime {
            self.scale.clone() * curve.y(self.t())
        } else {
            self.scale.clone()
        }
    }
}

#[derive(Debug, Clone)]
struct ParticlePool {
    particles: Vec<Particle>,
    free: Vec<usize>,
    init: bool,
}

impl Default for ParticlePool {
    fn default() -> Self {
        Self {
            particles: vec![],
            free: vec![],
            init: false,
        }
    }
}

impl ParticlePool {
    /// Initiate a bunch of dead particles.
    fn of_size(nb: usize) -> Self {
        Self {
            particles: (0..nb).map(|_| Particle::default()).collect(),
            free: (0..nb).collect(),
            init: true,
        }
    }

    /// Return the first available particle.
    fn get_available(&mut self) -> Option<&mut Particle> {
        let particles = &mut self.particles;
        self.free
            .pop()
            .map(move |idx| unsafe { particles.get_unchecked_mut(idx) })
    }

    fn all_dead(&self) -> bool {
        self.particles.len() == self.free.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmitterSource {
    /// Spawn particle from this point
    Point,

    /// Spawn particle randomly on this line
    /// Line relative to emitter's transform, so first point will be transform + v1, next point will be
    /// transform + v2
    Line(Vector2f, Vector2f),
}

impl EmitterSource {
    fn spawn_position<R: Rng>(&self, emitter_position: &Vector2f, rand: &mut R) -> Vector2f {
        match self {
            Self::Point => emitter_position.clone(),
            Self::Line(p1, p2) => (emitter_position.clone() - p1)
                .lerp(&(*emitter_position + p2), rand.gen_range(0.0, 1.0f32)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticleShape {
    Quad,
    Texture(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleEmitter {
    pub enabled: bool,

    #[serde(skip)]
    particles: ParticlePool,
    pub source: EmitterSource,
    pub shape: ParticleShape,

    pub damping: f32,

    pub velocity_range: (f32, f32),
    pub angle_range: (f32, f32),

    pub scale: ParticleScale,
    pub scale_over_lifetime: Option<Curve<f32>>,

    /// Particle per frame to emit.
    pub particle_number: f32,

    /// when particle_number < 1, we need to know when we should spawn a particle.
    #[serde(skip)]
    pub nb_accumulator: f32,

    /// Color of the particle
    pub colors: Curve<RgbaColor>,

    /// How long does the particle (in frames)
    #[serde(default)]
    pub particle_life: u32,

    /// Offset applied to a particle position on spawn.
    #[serde(default)]
    pub position_offset: Vector2f,

    /// If true, only spawn stuff once
    #[serde(default)]
    pub burst: bool,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            enabled: true,
            damping: 0.0,
            particles: Default::default(),
            source: EmitterSource::Point,
            shape: ParticleShape::Quad,
            velocity_range: (0.0, 10.0),
            angle_range: (0.0, 2.0 * std::f32::consts::PI),
            scale: ParticleScale::Constant(Vector2f::new(5.0, 5.0)),
            scale_over_lifetime: None,
            particle_number: 1.0,
            nb_accumulator: 0.0,
            colors: Curve {
                xs: vec![0.0],
                ys: vec![colors::RED],
            },
            particle_life: 10,
            position_offset: Default::default(),
            burst: false,
        }
    }
}

impl ParticleEmitter {
    pub fn load_from_path<P: AsRef<Path>>(p: P) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(p)?;
        let mut emitter: Self = serde_json::from_str(&content)?;
        emitter.init_pool();
        Ok(emitter)
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Necessary when getting the emitter from a file.
    pub fn init_pool(&mut self) {
        let frame_needed = if self.burst {
            1
        } else {
            self.particle_life as usize + 1
        };
        self.particles = ParticlePool::of_size(self.particle_number.ceil() as usize * frame_needed);
    }

    /// Update the position and velocity of all particles. If a particle is dead, respawn it :)
    /// Return true if should despawn the particle emitter.
    fn update(&mut self, position: &Vector2f, dt: f32) -> bool {
        if !self.particles.init {
            self.init_pool()
        }
        let mut rng = rand::thread_rng();

        // emit particles.
        trace!(
            "Will emit {} particles (Acc = {}",
            self.particle_number,
            self.nb_accumulator
        );
        self.nb_accumulator += self.particle_number;

        let entire_nb = self.nb_accumulator.floor() as u32;
        if entire_nb > 0 {
            if self.enabled {
                for _ in 0..entire_nb {
                    if let Some(particle) = self.particles.get_available() {
                        trace!("Emit particle");

                        let angle = rng.gen_range(self.angle_range.0, self.angle_range.1);
                        let rotation = Rotation2::new(angle);
                        let speed = rng.gen_range(self.velocity_range.0, self.velocity_range.1);

                        // PARTICLE SCALE. -> initial scale.
                        let scale = match &self.scale {
                            ParticleScale::Constant(s) => s.clone(),
                            ParticleScale::Random(low, high) => {
                                let x = rng.gen_range(low.x, high.x);
                                let y = rng.gen_range(low.y, high.y);
                                Vector2f::new(x, y)
                            }
                        };

                        particle.respawn(
                            self.particle_life,
                            self.source.spawn_position(position, &mut rng)
                                + self.position_offset.clone(),
                            rotation * (Vector2f::new(speed, 0.0)),
                            scale.clone(),
                            self.damping,
                            self.scale_over_lifetime.clone(),
                            angle,
                        );
                        particle.colors = self.colors.clone();
                        trace!("{:?}", particle);
                    }
                }
            }
            self.nb_accumulator -= self.nb_accumulator.floor();
        }

        // update existing particles.
        for (idx, p) in self.particles.particles.iter_mut().enumerate() {
            if p.alive() {
                p.update(dt);
            } else {
                if !self.particles.free.contains(&idx) {
                    self.particles.free.push(idx);
                }
            }
        }

        if self.burst {
            self.disable();

            if self.particles.all_dead() {
                return false;
            }
        }

        true
    }
}

const VS: &'static str = include_str!("particle-vs.glsl");
const FS: &'static str = include_str!("particle-fs.glsl");
const FS_TEXTURE: &'static str = include_str!("particle-texture-fs.glsl");

pub fn new_shader(surface: &mut super::Context) -> Program<(), (), ParticleShaderInterface> {
    surface
        .new_shader_program::<(), (), ParticleShaderInterface>()
        .from_strings(VS, None, None, FS)
        .expect("Program creation")
        .ignore_warnings()
}

pub fn new_texture_shader(
    surface: &mut super::Context,
) -> Program<(), (), TextureParticleShaderInterface> {
    surface
        .new_shader_program::<(), (), TextureParticleShaderInterface>()
        .from_strings(VS, None, None, FS_TEXTURE)
        .expect("Program creation")
        .ignore_warnings()
}

#[derive(UniformInterface)]
pub struct ParticleShaderInterface {
    pub projection: Uniform<[[f32; 4]; 4]>,
    #[uniform(unbound)]
    pub view: Uniform<[[f32; 4]; 4]>,
    pub model: Uniform<[[f32; 4]; 4]>,
    pub color: Uniform<[f32; 4]>,
}

#[derive(UniformInterface)]
pub struct TextureParticleShaderInterface {
    pub projection: Uniform<[[f32; 4]; 4]>,
    #[uniform(unbound)]
    pub view: Uniform<[[f32; 4]; 4]>,
    pub model: Uniform<[[f32; 4]; 4]>,
    pub color: Uniform<[f32; 4]>,

    /// Texture for the sprite.
    tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
}

pub struct ParticleSystem {
    tess: Tess<()>,
    shader: Program<(), (), ParticleShaderInterface>,
    texture_shader: Program<(), (), TextureParticleShaderInterface>,
}

impl ParticleSystem {
    pub fn new(surface: &mut super::Context) -> Self {
        let tess = surface
            .new_tess()
            .set_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .expect("Tess creation");
        Self {
            tess,
            shader: new_shader(surface),
            texture_shader: new_texture_shader(surface),
        }
    }

    pub fn update<GE>(&mut self, world: &World, dt: Duration, resources: &Resources)
    where
        GE: CustomGameEvent,
    {
        let mut chan = resources.fetch_mut::<EventQueue<GE>>().unwrap();
        let mut remove_events = vec![];
        for (e, (t, emitter)) in world.query::<(&Transform, &mut ParticleEmitter)>().iter() {
            if !emitter.update(&t.translation, dt.as_secs_f32()) {
                chan.single_write(GameEvent::Delete(e));
            }
        }
        chan.drain_vec_write(&mut remove_events);
    }

    pub fn render(
        &mut self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate,
        projection: &Matrix4f,
        view: &Matrix4f,
        world: &World,
        textures: &mut AssetManager<SpriteAsset>,
    ) -> Result<(), PipelineError> {
        let tess = &self.tess;
        let render_st = RenderState::default()
            .set_depth_test(None)
            .set_blending(Blending {
                equation: Equation::Additive,
                src: Factor::One,
                dst: Factor::SrcAlphaComplement,
            });

        let view: [[f32; 4]; 4] = (*view).into();
        let projection: [[f32; 4]; 4] = (*projection).into();

        for (_, emitter) in world.query::<&mut ParticleEmitter>().iter() {
            match &emitter.shape {
                ParticleShape::Quad => {
                    shd_gate.shade(&mut self.shader, |mut iface, uni, mut rdr_gate| {
                        iface.set(&uni.projection, projection.into());
                        iface.set(&uni.view, view.into());

                        for p in &emitter.particles.particles {
                            if !p.alive() {
                                continue;
                            }

                            iface.set(&uni.color, p.color().to_normalized());
                            iface.set(&uni.model, to_model(&p.scale, p.rotation, &p.position));

                            rdr_gate.render(&render_st, |mut tess_gate| tess_gate.render(tess))?;
                        }

                        Ok(())
                    })?;
                }
                ParticleShape::Texture(id) => {
                    if let Some(tex) = textures.get_mut(&Handle(id.clone())) {
                        let mut res = Ok(());
                        let shader = &mut self.texture_shader;
                        tex.execute_mut(|asset| {
                            if let Some(tex) = asset.texture() {
                                let bound_tex = pipeline.bind_texture(tex).unwrap();
                                res = shd_gate.shade(shader, |mut iface, uni, mut rdr_gate| {
                                    iface.set(&uni.projection, projection);
                                    iface.set(&uni.view, view);
                                    iface.set(&uni.tex, bound_tex.binding());
                                    for p in &emitter.particles.particles {
                                        if !p.alive() {
                                            continue;
                                        }

                                        iface.set(&uni.color, p.color().to_normalized());
                                        iface.set(
                                            &uni.model,
                                            to_model(&p.scale, p.rotation, &p.position),
                                        );

                                        rdr_gate.render(&render_st, |mut tess_gate| {
                                            tess_gate.render(tess)
                                        })?;
                                    }

                                    Ok(())
                                });
                            }
                        });

                        res?;
                    } else {
                        debug!("Texture is not loaded {}", id);
                        textures.load(id.clone());
                    }
                }
            }
        }

        Ok(())
    }
}

fn to_model(scale: &Vector2f, rotation: f32, translation: &Vector2f) -> [[f32; 4]; 4] {
    //let rot_mat = Matrix4f::new_rotation(Vector3::new(0.0, 0.0, rotation));
    (Matrix4f::new_translation(&Vector3::new(translation.x, translation.y, 0.0))
        * Matrix4f::new_nonuniform_scaling(&Vector3::new(scale.x, scale.y, 0.0)))
    .into()
}
