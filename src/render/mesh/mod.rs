use crate::assets::shader::ShaderManager;
use crate::assets::sprite::SpriteAsset;
use crate::assets::{AssetManager, Handle};
use crate::core::colors::RgbaColor;
use crate::core::transform::Transform;
use crate::geom2::Matrix4f;
use crate::render::Context;
use instant::Instant;
use luminance::blending::{Blending, Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::pipeline::{PipelineError, TextureBinding};
use luminance::pixel::NormUnsigned;
use luminance::render_state::RenderState;
use luminance::shader::Uniform;
use luminance::tess::Mode;
use luminance::texture::Dim2;
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_front::shader::Program;
use luminance_front::{pipeline::Pipeline, shading_gate::ShadingGate, tess::Tess};

use serde_derive::{Deserialize, Serialize};

mod sprite_material;

// Vertex definition
// -----------------
// Just position, texture coordinates and color for 2D. No need
// for normal, tangent...
// -----------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "position", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,

    #[sem(name = "uv", repr = "[f32; 2]", wrapper = "TextureCoord")]
    TextureCoord,
    #[sem(name = "color", repr = "[f32; 4]", wrapper = "VertexColor")]
    Color,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Vertex, Copy, Debug, Clone)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex {
    /// Position of the vertex in 2D.
    position: VertexPosition,

    /// Texture coordinates for the vertex.
    uv: TextureCoord,

    /// Color for the vertex.
    color: VertexColor,
}

// Uniform definition
// ------------------
// Matrices to translate to view space, other useful uniforms such as timestamp, delta,
// and so on...
// --------------------------------------------------------------------------------------
#[allow(dead_code)]
#[derive(UniformInterface)]
pub struct ShaderUniform {
    /// PROJECTION matrix in MVP
    #[uniform(unbound, name = "u_projection")]
    projection: Uniform<[[f32; 4]; 4]>,
    /// VIEW matrix in MVP
    #[uniform(unbound, name = "u_view")]
    view: Uniform<[[f32; 4]; 4]>,
    /// MODEL matrix in MVP
    #[uniform(unbound, name = "u_model")]
    model: Uniform<[[f32; 4]; 4]>,
    /// Texture for the sprite.
    #[uniform(unbound, name = "u_tex_1")]
    tex_1: Uniform<TextureBinding<Dim2, NormUnsigned>>,
    /// true if should blink.
    #[uniform(unbound, name = "u_time")]
    time: Uniform<f32>,
    /// Sprite number in the spritesheet
    #[uniform(unbound, name = "u_sprite_nb")]
    sprite_number: Uniform<f32>,
    /// Column in the spritesheet
    #[uniform(unbound, name = "u_columns")]
    spritesheet_columns: Uniform<f32>,
    /// Sprite number in the spritesheet
    #[uniform(unbound, name = "u_rows")]
    spritesheet_rows: Uniform<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Material {
    /// Will use the given vertex and fragment shaders for the mesh.
    Shader {
        vertex_shader_id: String,
        fragment_shader_id: String,
    },
    Sprite {
        /// Texture ID
        sprite_id: String,
        /// Sprite number if spritesheet
        sprite_nb: u32,
        /// Number of columns for spritesheet
        columns: u32,
        /// Number of rows for spritesheet
        rows: u32,
    },
}

impl Material {
    pub fn material_id(&self) -> u16 {
        match self {
            Material::Sprite { .. } => 1,
            // Should probably have a different ID for different shaders...
            Material::Shader { .. } => 2,
        }
    }
}

/// Render meshes with materials.
pub struct MeshRenderer {
    tess: Tess<Vertex, u32>,
    /// used to send elapsed time to shader.
    creation_time: Instant,

    /// shader for sprites.
    sprite_shader: Program<VertexSemantics, (), ShaderUniform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshRender {
    pub enabled: bool,
    pub material: Material,
    /// Depth of the material. Larger depth will be renderer first.
    pub depth: u16,
}

impl MeshRender {
    fn sorting_key(&self) -> u32 {
        let high = (self.depth as u32) << 16;
        let low = self.material.material_id() as u32;
        high + low
    }
}

impl MeshRenderer {
    pub fn new(surface: &mut Context) -> Self {
        let color = RgbaColor::new(255, 0, 0, 255).to_normalized();

        let (vertices, indices) = (
            vec![
                Vertex {
                    position: VertexPosition::new([-1.0, -1.0]),
                    uv: TextureCoord::new([0.0, 0.0]),
                    color: VertexColor::new(color),
                },
                Vertex {
                    position: VertexPosition::new([-1.0, 1.0]),
                    uv: TextureCoord::new([0.0, 1.0]),
                    color: VertexColor::new(color),
                },
                Vertex {
                    position: VertexPosition::new([1.0, 1.0]),
                    uv: TextureCoord::new([1.0, 1.0]),
                    color: VertexColor::new(color),
                },
                Vertex {
                    position: VertexPosition::new([1.0, -1.0]),
                    uv: TextureCoord::new([1.0, 0.0]),
                    color: VertexColor::new(color),
                },
            ],
            vec![0, 1, 2, 0, 2, 3],
        );

        let tess = surface
            .new_tess()
            .set_mode(Mode::Triangle)
            .set_indices(indices)
            .set_vertices(vertices)
            .build()
            .unwrap();

        Self {
            tess,
            creation_time: Instant::now(),
            sprite_shader: sprite_material::new_shader(surface),
        }
    }
    pub fn render(
        &mut self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate,
        proj_matrix: &Matrix4f,
        view: &Matrix4f,
        world: &hecs::World,
        shader_manager: &mut ShaderManager,
        textures: &mut AssetManager<SpriteAsset>,
    ) -> Result<(), PipelineError> {
        // let handle = Handle(("simple-vs.glsl".to_string(), "simple-fs.glsl".to_string()));

        let render_st = RenderState::default()
            .set_depth_test(None)
            .set_blending_separate(
                Blending {
                    equation: Equation::Additive,
                    src: Factor::SrcAlpha,
                    dst: Factor::SrcAlphaComplement,
                },
                Blending {
                    equation: Equation::Additive,
                    src: Factor::One,
                    dst: Factor::Zero,
                },
            );
        let elapsed = self.creation_time.elapsed().as_secs_f32();

        let mut query = world.query::<(&Transform, &MeshRender)>();
        let mut to_render = query
            .iter()
            .filter(|(_, (_, r))| r.enabled)
            .collect::<Vec<_>>();
        to_render.sort_by(|(_, (_, a)), (_, (_, b))| a.sorting_key().cmp(&b.sorting_key()));

        //[[f32; 4]; 4]
        let view: [[f32; 4]; 4] = (*view).into();
        let proj_matrix: [[f32; 4]; 4] = (*proj_matrix).into();

        for (_, (t, render)) in to_render {
            let model: [[f32; 4]; 4] = t.to_model().into();
            let quad = &self.tess;

            match render.material {
                Material::Shader {
                    ref vertex_shader_id,
                    ref fragment_shader_id,
                } => {
                    let handle = Handle((vertex_shader_id.clone(), fragment_shader_id.clone()));
                    if let Some(shader) = shader_manager.get_mut(&handle) {
                        if let Some(ret) = shader.execute_mut(|shader_asset| {
                            if let Some(ref mut shader) = shader_asset.shader {
                                shd_gate.shade(shader, |mut iface, uni, mut rdr_gate| {
                                    iface.set(&uni.time, elapsed);
                                    iface.set(&uni.projection, proj_matrix);
                                    iface.set(&uni.view, view);
                                    iface.set(&uni.model, model);
                                    rdr_gate
                                        .render(&render_st, |mut tess_gate| tess_gate.render(quad))
                                })
                            } else {
                                Ok(())
                            }
                        }) {
                            ret?;
                        }
                    } else {
                        shader_manager.load(handle.0);
                    }
                }
                Material::Sprite {
                    ref sprite_id,
                    sprite_nb,
                    columns,
                    rows,
                } => {
                    let shader = &mut self.sprite_shader;
                    shd_gate.shade(shader, |mut iface, uni, mut rdr_gate| {
                        iface.set(&uni.projection, proj_matrix);
                        iface.set(&uni.view, view);
                        iface.set(&uni.model, model);
                        iface.set(&uni.sprite_number, sprite_nb as f32);
                        iface.set(&uni.spritesheet_columns, columns as f32);
                        iface.set(&uni.spritesheet_rows, rows as f32);
                        if let Some(tex) = textures.get_mut(&Handle(sprite_id.clone())) {
                            let mut res = Ok(());
                            tex.execute_mut(|asset| {
                                if let Some(tex) = asset.texture() {
                                    let bound_tex = pipeline.bind_texture(tex);
                                    match bound_tex {
                                        Ok(bound_tex) => {
                                            iface.set(&uni.tex_1, bound_tex.binding());
                                            res = rdr_gate.render(&render_st, |mut tess_gate| {
                                                tess_gate.render(quad)
                                            });
                                        }
                                        Err(e) => {
                                            res = Err(e);
                                        }
                                    }
                                }
                            });

                            res?;
                        } else {
                            debug!("Texture is not loaded {}", sprite_id);
                            textures.load(sprite_id.clone());
                        }

                        Ok(())
                    })?;
                }
            }
        }

        Ok(())
    }
}
