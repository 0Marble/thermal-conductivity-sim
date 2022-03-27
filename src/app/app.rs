use std::rc::Rc;
use std::time::Duration;

use crate::app::model_manager::ModelManager;
use crate::ticker::Ticker;
use crate::{call, window::window::Window};

use super::model_manager::ModelInfo;
use super::ui::*;
use crate::renderer::{
    error::Error, renderer::BatchRenderer, shader::Shader, vertex::VertexLayout,
};
use nalgebra::Matrix4;

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

fn get_node_color(node: f64) -> (f32, f32, f32, f32) {
    (node as f32 / 100., 0., 0., 1.)
}

fn nodes_to_verts(
    nodes: &[f64],
    length: f64,
    height: f32,
    offset: (f32, f32),
    index_offset: u16,
) -> (Vec<f32>, Vec<u16>) {
    let mut inds = vec![];
    let mut verts = vec![];

    let node_count = nodes.len();

    let (x, y) = offset;

    let left = -length as f32 / 2. + x;
    let step = length as f32 / (node_count as f32 - 1.);
    let top = -height / 2. + y;
    let bottom = height / 2. + y;

    let mut i = 0;
    for node in nodes {
        let (r, g, b, a) = get_node_color(*node);
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

pub struct UiReducer {
    model_manager: Rc<ModelManager>,
    model_info: Rc<Vec<ModelInfo>>,
    tps: usize,
}

impl UiReducer {
    pub fn new(model_manager: Rc<ModelManager>) -> Self {
        Self {
            model_manager,
            model_info: Rc::new(Vec::new()),
            tps: 0,
        }
    }

    pub fn set_model_info(&mut self, model_info: (Vec<ModelInfo>, usize)) {
        let (model_info, tps) = model_info;
        self.model_info = Rc::new(model_info);
        self.tps = tps;
    }
}

impl Reducer<UiPost, UiGet> for UiReducer {
    fn reduce(&mut self, op: UiPost) {
        match op {
            UiPost::AddModel(n, m) => {
                self.model_manager.add_model(&n, m);
            }
            UiPost::RestartModel(s) => {
                self.model_manager.restart_model(&s);
            }
            UiPost::RemoveModel(n) => self.model_manager.remove_model(&n),
            UiPost::StartComparison(n1, n2) => self.model_manager.start_comparison(&n1, &n2),
            UiPost::StopComparison(n1, n2) => self.model_manager.stop_comparison(&n1, &n2),
            UiPost::SetMinFrameTime(_) => {}
            UiPost::SetMinTickTime(d) => {
                self.model_manager.set_min_tick_time(d);
            }
        }
    }

    fn request(&mut self, op: &mut UiGet) {
        match op {
            UiGet::ModelInfo(None) => {
                *op = UiGet::ModelInfo(Some(self.model_info.clone()));
            }
            UiGet::GetFps(None) => *op = UiGet::GetFps(Some(120)),
            UiGet::GetTps(None) => *op = UiGet::GetTps(Some(self.tps)),
            _ => (),
        }
    }
}

pub struct App {
    window: Window,
    renderer: BatchRenderer<gl::types::GLfloat, gl::types::GLushort>,
    shader: Shader,

    ticker: Ticker,
    model_manager: Rc<ModelManager>,

    ui: Controls,
    reducer: UiReducer,
    is_running: bool,
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

        let model_manager = Rc::new(ModelManager::new(Duration::from_micros(100)));

        Ok(Self {
            is_running: true,
            shader,
            renderer,
            window,
            ticker: Ticker::new(Duration::from_millis(7)),
            ui: Controls::new(),
            reducer: UiReducer::new(model_manager.clone()),
            model_manager,
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        while call!(self.window.process_events())? && self.is_running {
            self.ticker.start_tick();

            let (model_info, tps) = self.model_manager.get_info();
            let mut offset = 0;
            for (i, m) in model_info.iter().enumerate() {
                let n = &m.nodes;
                let l = &m.length;
                let (v, i) = nodes_to_verts(&n[..], *l, 30., (0., -100. + i as f32 * 35.), offset);
                offset += n.len() as u16;
                call!(self.renderer.push(&v[..], &i[..]))?;
            }
            self.reducer.set_model_info((model_info, tps));

            call!(self.window.start_frame())?;
            call!(self.renderer.draw(&self.shader, gl::TRIANGLES))?;
            self.ui
                .draw(&mut self.window.egui_context, &mut self.reducer);

            call!(self.window.end_frame())?;
            call!(self.renderer.clear())?;

            self.ticker.end_tick();
        }

        Ok(())
    }
}
