use super::ShaderUniform;
use crate::render::mesh::VertexSemantics;
use crate::render::Context;
use luminance_front::context::GraphicsContext;
use luminance_front::shader::Program;

const SPRITE_VS: &'static str = include_str!("sprite-vs.glsl");
const SPRITE_FS: &'static str = include_str!("sprite-fs.glsl");

pub fn new_shader(surface: &mut Context) -> Program<VertexSemantics, (), ShaderUniform> {
    surface
        .new_shader_program::<VertexSemantics, (), ShaderUniform>()
        .from_strings(SPRITE_VS, None, None, SPRITE_FS)
        .expect("Program creation")
        .ignore_warnings()
}
