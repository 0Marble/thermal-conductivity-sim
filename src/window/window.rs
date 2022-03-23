use egui::CtxRef;
use egui_sdl2_gl::painter::Painter;
use egui_sdl2_gl::{self, EguiStateHandler};

extern crate gl;
extern crate sdl2;
use sdl2::event::Event;

use crate::renderer::error::Error;
use crate::{call, gl_call};

pub struct Window {
    window: sdl2::video::Window,
    event_pump: sdl2::EventPump,
    _sdl_context: sdl2::Sdl,
    _video_subsystem: sdl2::VideoSubsystem,
    _gl_context: sdl2::video::GLContext,
    painter: Painter,
    egui_state: EguiStateHandler,
    pub egui_context: CtxRef,
    pub is_running: bool,
}

impl Window {
    pub fn new(width: u32, height: u32, title: &str) -> Result<Self, Error> {
        let sdl_context = call!(sdl2::init())?;
        let video_subsystem = call!(sdl_context.video())?;

        let gl_attributes = video_subsystem.gl_attr();
        gl_attributes.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attributes.set_double_buffer(true);
        gl_attributes.set_multisample_samples(4);
        gl_attributes.set_framebuffer_srgb_compatible(true);
        // gl_attributes.set_context_version(4, 5);

        let window = call!(video_subsystem
            .window(title, width, height)
            .opengl()
            .resizable()
            .build())?;

        let gl_context = call!(window.gl_create_context())?;
        // gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);
        call!(window
            .subsystem()
            .gl_set_swap_interval(sdl2::video::SwapInterval::VSync))?;

        let event_pump = call!(sdl_context.event_pump())?;

        let (painter, egui_state) = egui_sdl2_gl::with_sdl2(
            &window,
            egui_sdl2_gl::ShaderVersion::Default,
            egui_sdl2_gl::DpiScaling::Custom(2.),
        );

        let egui_context = egui::CtxRef::default();

        Ok(Self {
            egui_context,
            egui_state,
            painter,
            _gl_context: gl_context,
            _video_subsystem: video_subsystem,
            _sdl_context: sdl_context,
            window,
            event_pump,
            is_running: true,
        })
    }

    pub fn start_frame(&mut self) -> Result<(), Error> {
        self.egui_context.begin_frame(self.egui_state.input.take());

        gl_call!(gl::ClearColor(0.5, 0.5, 0.5, 1.))?;
        gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT))?;

        Ok(())
    }

    pub fn end_frame(&mut self) -> Result<(), Error> {
        let (egui_output, draw_commands) = self.egui_context.end_frame();
        self.egui_state.process_output(&self.window, &egui_output);
        self.painter.paint_jobs(
            None,
            self.egui_context.tessellate(draw_commands),
            &self.egui_context.font_image(),
        );

        self.window.gl_swap_window();
        Ok(())
    }

    pub fn process_events(&mut self) -> Result<bool, Error> {
        if !self.is_running {
            return Ok(false);
        }

        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(false),
                _ => self
                    .egui_state
                    .process_input(&self.window, event, &mut self.painter),
            }
        }

        Ok(true)
    }
}
