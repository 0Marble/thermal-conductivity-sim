use crate::model::analytic::AnalyticModel;
use crate::model::differential::DifferentialModel;
use crate::{call, model::model::Model, window::window::Window};

use crate::app::ui::{Info, ModelControls};
use crate::renderer::{
    error::Error, renderer::BatchRenderer, shader::Shader, vertex::VertexLayout,
};
use nalgebra::Matrix4;
use rayon::iter::IntoParallelIterator;

use rayon::prelude::*;
use std::sync;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const VERT_SRC: &'static str = r#"
#version 400 core
layout(location = 0) in vec4 vertInPosition;
layout(location = 1) in vec4 vertInColor; 
uniform mat4 uMVP;

out VertexData 
{
    vec4 position;
    vec4 color;
} vertOut;

void main()
{
    gl_Position = uMVP * vertInPosition;

    vertOut.color = vertInColor;
    vertOut.position = vertInPosition;
}
"#;

const FRAG_SRC: &'static str = r#"#version 400 core

in VertexData
{
    vec4 position;
    vec4 color;
} fragIn;
out vec4 color;

void main()
{
    color = fragIn.color;
}"#;

type T = f64;
pub struct App {
    window: Window,

    diff_model: sync::Arc<sync::Mutex<Option<DifferentialModel>>>,
    analytic_model: sync::Arc<sync::Mutex<Option<AnalyticModel>>>,
    model_difference: Option<T>,

    physics_thread: Option<thread::JoinHandle<()>>,
    physics_thread_trasmitter: mpsc::Sender<MessageToThread>,
    physics_thread_receiver: mpsc::Receiver<MessageFromThread>,

    renderer: BatchRenderer<gl::types::GLfloat, gl::types::GLushort>,
    shader: Shader,

    controls: ModelControls,
    info: Info,

    fps: u64,
    min_frame_time: u64,

    tps: sync::Arc<sync::Mutex<u64>>,
    min_tick_time: sync::Arc<sync::Mutex<u64>>,
}

enum MessageToThread {
    Exit,
    FrameEnded,
    FrameStarted,
}

enum MessageFromThread {
    Waiting,
}

impl App {
    pub fn new() -> Result<Self, Error> {
        let window = call!(Window::new(640, 480, "Hello"))?;

        let mvp: Matrix4<f32> = Matrix4::new_orthographic(-320., 320., 240., -240., 0., -1.);
        let mut shader = call!(Shader::new(&[
            (VERT_SRC, gl::VERTEX_SHADER),
            (FRAG_SRC, gl::FRAGMENT_SHADER),
        ]))?;
        call!(shader.set_uniform4x4("uMVP", &mvp))?;

        let mut layout = VertexLayout::new();
        call!(layout.push_attribute(gl::FLOAT, 2, false, 0))?;
        call!(layout.push_attribute(gl::FLOAT, 4, false, 1))?;

        let renderer: BatchRenderer<gl::types::GLfloat, gl::types::GLushort> =
            call!(BatchRenderer::new(
                layout,
                None,
                None,
                u16::MAX as i32,
                u16::MAX as i32,
                gl::STATIC_DRAW,
                gl::UNSIGNED_SHORT,
            ))?;

        let diff_model: sync::Arc<sync::Mutex<Option<DifferentialModel>>> =
            sync::Arc::new(sync::Mutex::new(None));
        let analytic_model: sync::Arc<sync::Mutex<Option<AnalyticModel>>> =
            sync::Arc::new(sync::Mutex::new(None));

        let diff_copy = diff_model.clone();
        let analytic_copy = analytic_model.clone();
        let (physics_thread_trasmitter, rx) = mpsc::channel::<MessageToThread>();
        let (tx, physics_thread_receiver) = mpsc::channel::<MessageFromThread>();

        let tps = sync::Arc::new(sync::Mutex::new(0));
        let min_tick_time = sync::Arc::new(sync::Mutex::new(10));
        let min_tick_time_clone = min_tick_time.clone();
        let tps_clone = tps.clone();

        let physics_thread = thread::spawn(move || {
            let rx = rx;
            let tx = tx;

            let mut last_tps = Instant::now();
            let mut last_tick = Instant::now();
            let min_tick_time = min_tick_time_clone;
            let mut tick_count = 0;
            let tps = tps_clone;

            'main: loop {
                match rx.try_recv() {
                    Ok(msg) => match msg {
                        MessageToThread::Exit => break 'main,
                        MessageToThread::FrameStarted => {
                            tx.send(MessageFromThread::Waiting).unwrap();
                            loop {
                                match rx.recv().unwrap() {
                                    MessageToThread::Exit => break 'main,
                                    MessageToThread::FrameEnded => break,
                                    MessageToThread::FrameStarted => (),
                                }
                            }
                        }
                        MessageToThread::FrameEnded => {}
                    },
                    Err(_) => (),
                }

                let now = Instant::now();
                let tick_time = now
                    .checked_duration_since(last_tick)
                    .map(|d| d.as_micros())
                    .unwrap_or(0) as u64;
                let min_tick_time = min_tick_time.lock().unwrap();
                if tick_time < *min_tick_time {
                    thread::sleep(Duration::from_micros(*min_tick_time - tick_time));
                }

                last_tick = Instant::now();

                let mut diff_model = diff_copy.lock().unwrap();
                let mut analytic_model = analytic_copy.lock().unwrap();

                match &mut *diff_model {
                    Some(m) => m.run_step(),
                    None => (),
                }
                match &mut *analytic_model {
                    Some(m) => m.run_step(),
                    None => (),
                }

                tick_count += 1;
                if now
                    .checked_duration_since(last_tps)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
                    >= 1
                {
                    *tps.lock().unwrap() = tick_count;
                    last_tps = Instant::now();
                    tick_count = 0;
                }
            }
        });

        Ok(Self {
            shader,
            renderer,
            window,

            controls: ModelControls::new(),
            info: Info::new(),

            analytic_model,
            diff_model,

            model_difference: None,

            physics_thread: Some(physics_thread),
            physics_thread_trasmitter,
            physics_thread_receiver,

            min_frame_time: 30,
            fps: 0,
            tps,
            min_tick_time,
        })
    }

    fn get_node_color(node: T) -> (f32, f32, f32, f32) {
        (node as f32 / 100., 0., 0., 1.)
    }

    fn draw_nodes(
        &self,
        nodes: &[T],
        length: T,
        offset: (f32, f32),
        index_offset: u16,
    ) -> (Vec<f32>, Vec<u16>) {
        let mut inds = vec![];
        let mut verts = vec![];

        let node_count = nodes.len();
        let height = 30.;

        let (x, y) = offset;

        let left = -length as f32 / 2. + x;
        let step = length as f32 / (node_count as f32 - 1.);
        let top = -height / 2. + y;
        let bottom = height / 2. + y;

        let mut i = 0;
        for node in nodes {
            let (r, g, b, a) = Self::get_node_color(*node);
            verts.push(left + i as f32 * step);
            verts.push(top);
            verts.push(r);
            verts.push(g);
            verts.push(b);
            verts.push(a);

            verts.push(left + i as f32 * step);
            verts.push(bottom);
            verts.push(r);
            verts.push(g);
            verts.push(b);
            verts.push(a);
            i += 1;
        }

        for i in index_offset..(index_offset + node_count as u16 - 1) {
            inds.push(2 * i);
            inds.push(2 * i + 1);
            inds.push(2 * i + 2);
            inds.push(2 * i + 2);
            inds.push(2 * i + 3);
            inds.push(2 * i + 1);
        }

        (verts, inds)
    }

    fn draw(&mut self) -> Result<(), Error> {
        let diff_model = &mut *call!(self.diff_model.lock())?;
        let analytic_model = &mut *call!(self.analytic_model.lock())?;

        call!(self.renderer.clear())?;

        match diff_model {
            None => (),
            Some(m) => {
                let (verts, inds) =
                    self.draw_nodes(m.get_cur_nodes(), *m.get_length(), (0., -50.), 0);
                call!(self.renderer.push(&verts[..], &inds[..]))?;
            }
        }

        match analytic_model {
            None => (),
            Some(m) => {
                let (verts, inds) = self.draw_nodes(
                    m.get_cur_nodes(),
                    *m.get_length(),
                    (0., 50.),
                    m.get_cur_nodes().len() as u16,
                );
                call!(self.renderer.push(&verts[..], &inds[..]))?;
            }
        }

        call!(self.window.start_frame())?;
        call!(self.renderer.draw(&self.shader, gl::TRIANGLES))?;

        egui::Window::new("Config").show(&self.window.egui_context, |ui| {
            self.controls.draw(
                ui,
                diff_model,
                analytic_model,
                &mut *self.min_tick_time.lock().unwrap(),
                &mut self.min_frame_time,
            )
        });

        egui::Window::new("Info").show(&self.window.egui_context, |ui| {
            self.info.draw(
                ui,
                self.fps as u32,
                *self.tps.lock().unwrap() as u32,
                self.model_difference,
                diff_model.as_ref().map(|m| m.get_elapsed_time()),
                &mut self.window.is_running,
            )
        });

        call!(self.window.end_frame())?;

        self.model_difference = match diff_model {
            None => None,
            Some(m) => match analytic_model {
                None => None,
                Some(a) => {
                    let a_nodes = a.get_cur_nodes();
                    let m_nodes = m.get_cur_nodes();

                    Some(
                        a_nodes
                            .into_par_iter()
                            .zip(m_nodes.into_par_iter())
                            .map(|(a, b)| (a - b) * (a - b))
                            .sum::<T>()
                            .sqrt(),
                    )
                }
            },
        };

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Error> {
        let mut last_frame = Instant::now();
        let mut last_fps = Instant::now();
        let mut frame_count = 0;

        while call!(self.window.process_events())? {
            let now = Instant::now();
            let frame_time = now
                .checked_duration_since(last_frame)
                .unwrap_or(Duration::new(0, 1));
            if (frame_time.as_millis() as u64) < self.min_frame_time {
                thread::sleep(Duration::from_millis(
                    self.min_frame_time - frame_time.as_millis() as u64,
                ));
            }
            last_frame = now;

            call!(self
                .physics_thread_trasmitter
                .send(MessageToThread::FrameStarted))?;
            self.physics_thread_receiver.recv().unwrap();
            call!(self.draw())?;
            // self.model_difference = self.compare_models();
            call!(self
                .physics_thread_trasmitter
                .send(MessageToThread::FrameEnded))?;

            frame_count += 1;
            if now
                .checked_duration_since(last_fps)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                >= 1
            {
                self.fps = frame_count;
                frame_count = 0;
                last_fps = Instant::now();
            }
        }

        call!(self.physics_thread_trasmitter.send(MessageToThread::Exit))?;
        let t = self.physics_thread.take().unwrap();
        t.join().unwrap();
        Ok(())
    }
}
