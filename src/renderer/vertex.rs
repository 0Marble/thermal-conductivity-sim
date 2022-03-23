use super::error::Error;
use crate::{call, gl_call};
use egui_sdl2_gl::gl;

use core::ffi::c_void;
use std::mem;
use std::ptr;

struct Attribute {
    index: u32,
    size: i32,
    type_: u32,
    normalize: bool,
    offset: i32,
}

pub struct VertexLayout {
    attributes: Vec<Attribute>,
    vertex_size: i32,
}

pub fn get_type_size(type_: u32) -> i32 {
    match type_ {
        gl::FLOAT => std::mem::size_of::<f32>() as i32,
        gl::INT => std::mem::size_of::<i32>() as i32,
        gl::UNSIGNED_INT => std::mem::size_of::<u32>() as i32,
        gl::SHORT => std::mem::size_of::<i16>() as i32,
        gl::UNSIGNED_SHORT => std::mem::size_of::<u16>() as i32,
        gl::BYTE => std::mem::size_of::<i8>() as i32,
        gl::UNSIGNED_BYTE => std::mem::size_of::<u8>() as i32,
        _ => unreachable!(),
    }
}

impl VertexLayout {
    pub fn new() -> Self {
        Self {
            attributes: vec![],
            vertex_size: 0,
        }
    }

    pub fn push_attribute(
        &mut self,
        component_type: u32,
        component_count: i32,
        normalize: bool,
        index: u32,
    ) -> Result<(), Error> {
        self.attributes.push(Attribute {
            index,
            size: component_count,
            type_: component_type,
            normalize,
            offset: self.vertex_size,
        });
        self.vertex_size += get_type_size(component_type) * component_count;
        Ok(())
    }

    pub fn bind(&self) -> Result<(), Error> {
        for a in &self.attributes {
            gl_call!(gl::EnableVertexAttribArray(a.index))?;
            gl_call!(gl::VertexAttribPointer(
                a.index,
                a.size,
                a.type_,
                a.normalize as u8,
                self.vertex_size,
                a.offset as *const c_void,
            ))?;
        }

        Ok(())
    }

    pub fn unbind(&self) -> Result<(), Error> {
        for a in &self.attributes {
            gl_call!(gl::DisableVertexAttribArray(a.index))?
        }

        Ok(())
    }
}

pub trait Buffer {
    fn get_buffer_type() -> u32;
    fn get_buffer(&self) -> &gl::types::GLuint;

    fn create_buffer<T: std::fmt::Debug>(
        data: Option<&[T]>,
        count: Option<i32>,
        usage: u32,
    ) -> Result<gl::types::GLuint, Error> {
        let mut buffer: gl::types::GLuint = 0;
        let buffer_type = Self::get_buffer_type();
        gl_call!(gl::CreateBuffers(1, &mut buffer))?;
        gl_call!(gl::BindBuffer(buffer_type, buffer))?;

        if data.is_none() && count.is_none() {
            Err(Error::InvalidBuffer(format!(
                "Both data and vertex count are not set, {} line {}",
                file!(),
                line!()
            )))
        } else {
            let size = count
                .map(|c| c as isize)
                .unwrap_or(data.map(|d| d.len() as isize).unwrap())
                * mem::size_of::<T>() as isize;
            let raw_data = data
                .map(|d| d.as_ptr() as *const c_void)
                .unwrap_or(ptr::null());

            if count.is_some() {
                gl_call!(gl::BufferData(buffer_type, size, ptr::null(), usage))?;

                if data.is_some() {
                    gl_call!(gl::BufferSubData(
                        Self::get_buffer_type(),
                        0,
                        (data.unwrap().len() * std::mem::size_of::<T>()) as isize,
                        raw_data,
                    ))?;
                }
            } else {
                gl_call!(gl::BufferData(buffer_type, size, raw_data, usage))?;
            }
            Ok(buffer)
        }
    }

    fn bind(&self) -> Result<(), Error> {
        gl_call!(gl::BindBuffer(Self::get_buffer_type(), *self.get_buffer()))
    }

    fn unbind(&self) -> Result<(), Error> {
        gl_call!(gl::BindBuffer(Self::get_buffer_type(), 0))
    }

    fn set_buffer_data<T>(&self, data: &[T], element_offset: i32) -> Result<(), Error> {
        call!(self.bind())?;
        gl_call!(gl::BufferSubData(
            Self::get_buffer_type(),
            element_offset as isize * std::mem::size_of::<T>() as isize,
            data.len() as isize * std::mem::size_of::<T>() as isize,
            data.as_ptr() as *const c_void,
        ))
    }
}

pub struct VertexBuffer {
    buffer: gl::types::GLuint,
}

impl Buffer for VertexBuffer {
    fn get_buffer(&self) -> &gl::types::GLuint {
        &self.buffer
    }

    fn get_buffer_type() -> u32 {
        gl::ARRAY_BUFFER
    }
}

impl VertexBuffer {
    pub fn new<T: std::fmt::Debug>(
        data: Option<&[T]>,
        allocated_vertex_count: Option<i32>,
        usage: u32,
    ) -> Result<Self, Error> {
        Ok(Self {
            buffer: call!(Self::create_buffer(data, allocated_vertex_count, usage))?,
        })
    }
}

impl Drop for VertexBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.buffer);
        }
    }
}

pub struct IndexBuffer {
    buffer: gl::types::GLuint,
    index_type: u32,
}

impl Buffer for IndexBuffer {
    fn get_buffer(&self) -> &gl::types::GLuint {
        &self.buffer
    }

    fn get_buffer_type() -> u32 {
        gl::ELEMENT_ARRAY_BUFFER
    }
}

impl IndexBuffer {
    pub fn new<T: std::fmt::Debug>(
        data: Option<&[T]>,
        allocated_vertex_count: Option<i32>,
        usage: u32,
        index_type: u32,
    ) -> Result<Self, Error> {
        Ok(Self {
            buffer: call!(Self::create_buffer(data, allocated_vertex_count, usage))?,
            index_type,
        })
    }

    pub fn get_index_type(&self) -> &u32 {
        &self.index_type
    }
}

impl Drop for IndexBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.buffer);
        }
    }
}
