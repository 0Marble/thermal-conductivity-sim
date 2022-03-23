use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    GlError(String),
    ShaderCompilation(String),
    ShaderLinking(String),
    InvalidBuffer(String),
    BatchFull,
    At(String),
    UnknownUniform(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::GlError(e) => format!("GL_ERROR - {}", e),
                Self::ShaderCompilation(e) => format!("SHADER_COMPILE - {}", e),
                Self::ShaderLinking(e) => format!("SHADER_LINKING - {}", e),
                Self::InvalidBuffer(e) => format!("INVALID_BUFFER - {}", e),
                Self::BatchFull => format!("BATCH_FULL"),
                Self::At(e) => format!("{}", e),
                Self::UnknownUniform(e) => format!("UNKNOWN_UNIFORM {}", e),
            }
        )
    }
}

#[macro_export]
macro_rules! gl_call {
    ($func:expr) => {{
        unsafe {
            let res = $func;
            let err = gl::GetError();
            if err != 0 {
                Err(Error::GlError(format!(
                    "[{}] at {}, {}, line {}",
                    err,
                    stringify!($func),
                    file!(),
                    line!()
                )))
            } else {
                Ok(res)
            }
        }
    }};
}

#[macro_export]
macro_rules! call {
    ($func:expr) => {{
        match $func {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::At(format!(
                "{}\nAt {}, {}, line {}",
                e,
                stringify!($func),
                file!(),
                line!()
            ))),
        }
    }};
}
