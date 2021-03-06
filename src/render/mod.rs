use crate::assets::shader::ShaderManager;
use crate::assets::sprite::SpriteAsset;
use crate::assets::AssetManager;
use crate::core::camera::{ProjectionMatrix, VirtualDim};
use crate::render::mesh::MeshRenderer;
use crate::render::particle::ParticleSystem;
use crate::render::path::PathRenderer;
//use crate::render::sprite::SpriteRenderer;
use crate::core::window::WindowDim;
use crate::event::CustomGameEvent;
use crate::render::ui::{text, Gui, GuiContext, UiRenderer};
use crate::resources::Resources;
use glyph_brush::GlyphBrush;
use luminance::context::GraphicsContext;
use luminance::pipeline::{PipelineError, PipelineState, Render, Viewport};
use luminance::texture::Dim2;
use luminance_front::framebuffer::Framebuffer;
use std::time::Duration;

pub mod mesh;
pub mod particle;
pub mod path;
//pub mod sprite;
pub mod ui;

/// Build for desktop will use opengl
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub type Backend = luminance_gl::GL33;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub type Context = luminance_glfw::GlfwSurface;

/// Build for web (wasm) will use webgl
#[cfg(target_arch = "wasm32")]
pub type Backend = luminance_webgl::webgl2::WebGL2;
#[cfg(target_arch = "wasm32")]
pub type Context = luminance_web_sys::WebSysWebGL2Surface;

pub struct Renderer {
    /// Render sprites on screen.
    //sprite_renderer: SpriteRenderer,
    mesh_renderer: MeshRenderer,
    /// particles :)
    particle_renderer: ParticleSystem,
    ui_renderer: UiRenderer,
    path_renderer: PathRenderer,
}

impl Renderer {
    pub fn new(surface: &mut Context, gui_context: &GuiContext) -> Renderer {
        info!("Sprite renderer");
        //let sprite_renderer = sprite::SpriteRenderer::new(surface);
        info!("Particle renderer");
        let particle_renderer = ParticleSystem::new(surface);
        info!("GUI renderer");
        let ui_renderer = UiRenderer::new(surface, gui_context);
        let path_renderer = PathRenderer::new(surface);
        let mesh_renderer = MeshRenderer::new(surface);
        Self {
            //     sprite_renderer,
            mesh_renderer,
            particle_renderer,
            ui_renderer,
            path_renderer,
        }
    }

    pub fn prepare_ui(
        &mut self,
        surface: &mut Context,
        gui: Option<Gui>,
        resources: &Resources,
        fonts: &mut GlyphBrush<'static, text::Instance>,
    ) {
        self.ui_renderer.prepare(surface, gui, resources, fonts);
        self.path_renderer.prepare(surface, resources);
    }

    pub fn render(
        &mut self,
        surface: &mut Context,
        back_buffer: &mut Framebuffer<Dim2, (), ()>,
        world: &hecs::World,
        resources: &Resources,
    ) -> Render<PipelineError> {
        let projection_matrix = resources.fetch::<ProjectionMatrix>().unwrap().0.clone();
        let view = crate::core::camera::get_view_matrix(world).unwrap();

        let window_dim = resources.fetch::<WindowDim>().unwrap();
        let virtual_dim = resources.fetch::<VirtualDim>().unwrap();
        let aspect_ratio = virtual_dim.aspect();

        let w = window_dim.width;
        let h = window_dim.height;
        let (viewport_w, viewport_h, x, y) = if w as f32 > (h as f32 * aspect_ratio).ceil() {
            let (viewport_w, viewport_h) = ((h as f32 * aspect_ratio).ceil(), h as f32);
            let y = 0u32;
            let x = ((w as f32 - viewport_w) / 2.0).round() as u32;
            (viewport_w, viewport_h, x, y)
        } else {
            let (viewport_w, viewport_h) = (w as f32, (w as f32 / aspect_ratio).ceil());
            let y = ((h as f32 - viewport_h) / 2.0).round() as u32;
            let x = 0u32;
            (viewport_w, viewport_h, x, y)
        };

        //println!("w,h ({}, {})-> ({},{})", w, h, viewport_w, viewport_h);

        let mut textures = resources.fetch_mut::<AssetManager<SpriteAsset>>().unwrap();
        let mut shaders = resources.fetch_mut::<ShaderManager>().unwrap();
        surface
            .new_pipeline_gate()
            .pipeline(
                back_buffer,
                &PipelineState::default()
                    .set_viewport(Viewport::Specific {
                        x,
                        y,
                        width: viewport_w as u32,
                        height: viewport_h as u32,
                    })
                    .set_clear_color([0.0, 0.0, 0.0, 1.0]),
                |pipeline, mut shd_gate| {
                    // self.sprite_renderer.render(
                    //     &pipeline,
                    //     &mut shd_gate,
                    //     &projection_matrix,
                    //     &view,
                    //     &world,
                    //     &mut *textures,
                    // )?;

                    self.mesh_renderer.render(
                        &pipeline,
                        &mut shd_gate,
                        &projection_matrix,
                        &view,
                        &world,
                        &mut *shaders,
                        &mut *textures,
                    )?;

                    self.particle_renderer.render(
                        &pipeline,
                        &mut shd_gate,
                        &projection_matrix,
                        &view,
                        world,
                        &mut *textures,
                    )?;

                    self.ui_renderer.render(&pipeline, &mut shd_gate)?;
                    self.path_renderer
                        .render(&projection_matrix, &view, &mut shd_gate)
                },
            )
            .assume()
    }

    pub fn update<GE>(
        &mut self,
        _surface: &mut Context,
        world: &hecs::World,
        dt: Duration,
        resources: &Resources,
    ) where
        GE: CustomGameEvent,
    {
        // update particle systems.
        self.particle_renderer.update::<GE>(world, dt, resources);
    }
}
