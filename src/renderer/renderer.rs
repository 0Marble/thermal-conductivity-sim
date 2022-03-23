use super::{
    error::Error,
    shader::Shader,
    vertex::{get_type_size, Buffer, IndexBuffer, VertexBuffer, VertexLayout},
};
use crate::{call, gl_call};
use egui_sdl2_gl::gl;

use core::ffi::c_void;

pub fn draw(
    layout: &VertexLayout,
    vertices: &VertexBuffer,
    indices: &IndexBuffer,
    shader: &Shader,
    primitive: u32,
    count: i32,
    from_index: i32,
) -> Result<(), Error> {
    call!(shader.bind())?;
    call!(vertices.bind())?;
    call!(indices.bind())?;
    call!(layout.bind())?;
    let index_type = *indices.get_index_type();

    gl_call!(gl::DrawElements(
        primitive,
        count,
        index_type,
        (from_index * get_type_size(index_type)) as *const c_void
    ))?;
    vertices.unbind()
}

struct Batch<V: Clone, I: Clone> {
    vbo: VertexBuffer,
    ibo: IndexBuffer,

    new_vertices: Vec<V>,
    new_indices: Vec<I>,
    current_index_count: i32,
    current_vertex_count: i32,
    max_index_count: i32,
    max_vertex_count: i32,
}

impl<V: Clone + std::fmt::Debug, I: Clone + std::fmt::Debug> Batch<V, I> {
    pub fn new(
        vertices: Option<&[V]>,
        indices: Option<&[I]>,
        max_vertex_count: i32,
        max_index_count: i32,
        usage: u32,
        index_type: u32,
    ) -> Result<Self, Error> {
        Ok(Self {
            vbo: call!(VertexBuffer::new::<V>(
                vertices,
                Some(max_vertex_count),
                usage
            ))?,
            ibo: call!(IndexBuffer::new::<I>(
                indices,
                Some(max_index_count),
                usage,
                index_type
            ))?,
            new_indices: vec![],
            new_vertices: vec![],
            current_index_count: indices.unwrap_or(&[]).len() as i32,
            current_vertex_count: vertices.unwrap_or(&[]).len() as i32,
            max_index_count,
            max_vertex_count,
        })
    }

    pub fn get_empty_space(&self) -> (i32, i32) {
        (
            self.max_vertex_count - self.current_vertex_count - self.new_vertices.len() as i32,
            self.max_index_count - self.current_index_count - self.new_indices.len() as i32,
        )
    }

    pub fn push(&mut self, new_vertices: &[V], new_indices: &[I]) -> Result<(), Error> {
        if self.current_index_count + self.new_indices.len() as i32 + new_indices.len() as i32
            >= self.max_index_count
            || self.current_vertex_count
                + self.new_vertices.len() as i32
                + new_vertices.len() as i32
                >= self.max_vertex_count
        {
            Err(Error::BatchFull)
        } else {
            for v in new_vertices {
                self.new_vertices.push(v.clone());
            }
            for i in new_indices {
                self.new_indices.push(i.clone());
            }

            Ok(())
        }
    }

    pub fn draw(
        &mut self,
        layout: &VertexLayout,
        shader: &Shader,
        primitive: u32,
    ) -> Result<(), Error> {
        if !self.new_indices.is_empty() && !self.new_vertices.is_empty() {
            call!(self
                .vbo
                .set_buffer_data(&self.new_vertices[..], self.current_vertex_count))?;
            call!(self
                .ibo
                .set_buffer_data(&self.new_indices[..], self.current_index_count))?;

            self.current_index_count += self.new_indices.len() as i32;
            self.current_vertex_count += self.new_vertices.len() as i32;
            self.new_indices.clear();
            self.new_indices.clear();
        }

        draw(
            layout,
            &self.vbo,
            &self.ibo,
            shader,
            primitive,
            self.current_index_count,
            0,
        )
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        self.new_vertices.clear();
        self.new_indices.clear();

        self.current_index_count = 0;
        self.current_vertex_count = 0;

        Ok(())
    }
}

pub struct BatchRenderer<V: Clone, I: Clone> {
    layout: VertexLayout,
    batches: Vec<Batch<V, I>>,
    max_indices_per_batch: i32,
    max_vertices_per_batch: i32,
    usage: u32,
    index_type: u32,
}

impl<V: Clone + std::fmt::Debug, I: Clone + std::fmt::Debug> BatchRenderer<V, I> {
    pub fn new(
        layout: VertexLayout,
        vertices: Option<&[V]>,
        indices: Option<&[I]>,
        max_indices_per_batch: i32,
        max_vertices_per_batch: i32,
        usage: u32,
        index_type: u32,
    ) -> Result<Self, Error> {
        let mut s = Self {
            batches: vec![],
            layout,
            max_indices_per_batch,
            max_vertices_per_batch,
            usage,
            index_type,
        };

        call!(s.push(vertices.unwrap_or(&[]), indices.unwrap_or(&[])))?;

        Ok(s)
    }

    pub fn push(&mut self, vertices: &[V], indices: &[I]) -> Result<(), Error> {
        for b in &mut self.batches {
            let (v, i) = b.get_empty_space();
            if v > vertices.len() as i32 && i > indices.len() as i32 {
                call!(b.push(vertices, indices))?;
                return Ok(());
            }
        }

        self.batches.push(call!(Batch::new(
            Some(vertices),
            Some(indices),
            self.max_vertices_per_batch,
            self.max_indices_per_batch,
            self.usage,
            self.index_type,
        ))?);
        Ok(())
    }

    pub fn draw(&mut self, shader: &Shader, primitive: u32) -> Result<(), Error> {
        for b in &mut self.batches {
            call!(b.draw(&self.layout, shader, primitive))?;
        }

        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        for b in &mut self.batches {
            call!(b.clear())?;
        }

        Ok(())
    }
}
