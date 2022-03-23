use std::collections::HashMap;

use crate::{call, gl_call};
use egui_sdl2_gl::gl;
use nalgebra::Matrix4;

use std::ffi::CString;

use std::ptr;
use std::str;

pub struct Shader {
    program: gl::types::GLenum,

    uniforms: HashMap<String, gl::types::GLint>,
}

impl Shader {
    pub fn new(sources: &[(&str, u32)]) -> Result<Self, Error> {
        let program = gl_call!(gl::CreateProgram())?;

        let mut shaders = Vec::new();
        for (src, kind) in sources {
            let c_str = call!(CString::new(src.as_bytes()))?;

            let shader = gl_call!(gl::CreateShader(*kind))?;
            gl_call!(gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null()))?;
            gl_call!(gl::CompileShader(shader))?;

            let mut compile_status = gl::FALSE as gl::types::GLint;
            gl_call!(gl::GetShaderiv(
                shader,
                gl::COMPILE_STATUS,
                &mut compile_status
            ))?;

            if compile_status == (gl::TRUE as gl::types::GLint) {
                gl_call!(gl::AttachShader(program, shader))?;
                shaders.push(shader);
            } else {
                let mut info_log_length = 0;
                gl_call!(gl::GetShaderiv(
                    shader,
                    gl::INFO_LOG_LENGTH,
                    &mut info_log_length
                ))?;

                if info_log_length > 0 {
                    let mut buffer = Vec::with_capacity(info_log_length as usize);
                    buffer.resize((info_log_length - 1) as usize, 0);
                    gl_call!(gl::GetShaderInfoLog(
                        shader,
                        info_log_length,
                        ptr::null_mut(),
                        buffer.as_mut_ptr() as *mut gl::types::GLchar
                    ))?;

                    return Err(Error::ShaderCompilation(format!(
                        "{}",
                        str::from_utf8(&buffer[..]).unwrap_or("Unknown")
                    )));
                } else {
                    return Err(Error::ShaderCompilation("Unknown".to_owned()));
                }
            }
        }

        gl_call!(gl::LinkProgram(program))?;

        for s in shaders {
            gl_call!(gl::DetachShader(program, s))?;
            gl_call!(gl::DeleteShader(s))?;
        }

        gl_call!(gl::ValidateProgram(program))?;

        let mut link_status = gl::FALSE as gl::types::GLint;
        gl_call!(gl::GetProgramiv(program, gl::LINK_STATUS, &mut link_status))?;

        if link_status == (gl::TRUE as gl::types::GLint) {
            Ok(Self {
                program,
                uniforms: HashMap::new(),
            })
        } else {
            let mut info_log_length = 0;
            gl_call!(gl::GetProgramiv(
                program,
                gl::INFO_LOG_LENGTH,
                &mut info_log_length
            ))?;

            if info_log_length > 0 {
                let mut buffer = Vec::with_capacity(info_log_length as usize);
                buffer.resize((info_log_length - 1) as usize, 0);
                gl_call!(gl::GetProgramInfoLog(
                    program,
                    info_log_length,
                    ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut gl::types::GLchar
                ))?;

                return Err(Error::ShaderLinking(format!(
                    "{}",
                    str::from_utf8(&buffer[..]).unwrap_or("Unknown")
                )));
            } else {
                return Err(Error::ShaderLinking("Unknown".to_owned()));
            }
        }
    }

    pub fn get_program(&self) -> Option<&gl::types::GLenum> {
        Some(&self.program)
    }
}

use super::error::Error;

impl Shader {
    pub fn bind(&self) -> Result<(), Error> {
        gl_call!(gl::UseProgram(self.program))
    }

    pub fn unbind(&self) -> Result<(), Error> {
        gl_call!(gl::UseProgram(0))
    }

    pub fn set_uniform4x4(&mut self, uniform_name: &str, mat: &Matrix4<f32>) -> Result<(), Error> {
        call!(self.bind())?;

        if let Some(uniform_location) = self.uniforms.get(uniform_name) {
            gl_call!(gl::UniformMatrix4fv(
                *uniform_location,
                1,
                gl::FALSE,
                mat.as_slice().as_ptr(),
            ))
        } else {
            let c_str = call!(CString::new(uniform_name.as_bytes()))?;
            let uniform_location = gl_call!(gl::GetUniformLocation(self.program, c_str.as_ptr()))?;
            if uniform_location == -1 {
                Err(Error::UnknownUniform(format!("{}", uniform_name)))
            } else {
                self.uniforms
                    .insert(format!("{}", uniform_name), uniform_location);
                gl_call!(gl::UniformMatrix4fv(
                    uniform_location,
                    1,
                    gl::FALSE,
                    mat.as_slice().as_ptr(),
                ))
            }
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
        }
    }
}
