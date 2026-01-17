use glam::{Mat4, Vec3};
use std::ffi::CString;
use std::fs;
use std::ptr;
use std::str;

pub struct Shader {
    pub id: u32,
}

impl Shader {
    pub fn new(vertex_code: &str, fragment_code: &str) -> Result<Shader, String> {
        let vertex = compile_shader(vertex_code, gl::VERTEX_SHADER)?;
        let fragment = compile_shader(fragment_code, gl::FRAGMENT_SHADER)?;
        let id = link_program(vertex, fragment)?;
        Ok(Shader { id })
    }

    pub fn from_glsl(vertex_path: &str, fragment_path: &str) -> Result<Shader, String> {
        let vertex_code = fs::read_to_string(vertex_path)
            .map_err(|e| format!("Failed to read vertex shader file {}: {}", vertex_path, e))?;
        let fragment_code = fs::read_to_string(fragment_path).map_err(|e| {
            format!(
                "Failed to read fragment shader file {}: {}",
                fragment_path, e
            )
        })?;
        Shader::new(&vertex_code, &fragment_code)
    }

    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    // Helper to avoid repeated CString creation and to centralize GetUniformLocation.
    fn uniform_location(&self, name: &str) -> i32 {
        // It's rare that CString::new fails for shader uniform names (they come from literals/format),
        // so unwrap is acceptable here; keep it local to simplify callers.
        let c = CString::new(name).unwrap();
        unsafe { gl::GetUniformLocation(self.id, c.as_ptr()) }
    }

    pub fn set_bool(&self, name: &str, value: bool) {
        unsafe {
            gl::Uniform1i(self.uniform_location(name), value as i32);
        }
    }

    pub fn set_int(&self, name: &str, value: i32) {
        unsafe {
            gl::Uniform1i(self.uniform_location(name), value);
        }
    }

    pub fn set_float(&self, name: &str, value: f32) {
        unsafe {
            gl::Uniform1f(self.uniform_location(name), value);
        }
    }

    pub fn set_vec3(&self, name: &str, value: &Vec3) {
        unsafe {
            gl::Uniform3fv(self.uniform_location(name), 1, value.as_ref().as_ptr());
        }
    }

    pub fn set_mat4(&self, name: &str, value: &Mat4) {
        unsafe {
            gl::UniformMatrix4fv(
                self.uniform_location(name),
                1,
                gl::FALSE,
                value.as_ref().as_ptr(),
            );
        }
    }
}

fn compile_shader(code: &str, kind: u32) -> Result<u32, String> {
    let code_c = CString::new(code).map_err(|e| e.to_string())?;
    unsafe {
        let shader = gl::CreateShader(kind);
        gl::ShaderSource(shader, 1, &code_c.as_ptr(), ptr::null());
        gl::CompileShader(shader);
        check_compile_errors(shader, "SHADER")?;
        Ok(shader)
    }
}

fn link_program(vertex: u32, fragment: u32) -> Result<u32, String> {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vertex);
        gl::AttachShader(program, fragment);
        gl::LinkProgram(program);
        check_compile_errors(program, "PROGRAM")?;
        Ok(program)
    }
}

/// Improved and safe info-log retrieval.
/// This version queries the actual log length and allocates exactly that much memory,
/// avoiding unsafe manual set_len usage and ensuring we don't read uninitialized bytes.
fn check_compile_errors(obj: u32, type_: &str) -> Result<(), String> {
    unsafe {
        let mut success = gl::FALSE as i32;

        if type_ != "PROGRAM" {
            gl::GetShaderiv(obj, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                // Query info log length
                let mut len: i32 = 0;
                gl::GetShaderiv(obj, gl::INFO_LOG_LENGTH, &mut len);
                let buf_len = if len > 0 { len as usize } else { 1 };
                let mut buf: Vec<u8> = vec![0u8; buf_len];
                gl::GetShaderInfoLog(obj, len, ptr::null_mut(), buf.as_mut_ptr() as *mut i8);
                return Err(format!(
                    "ERROR::SHADER_COMPILATION_ERROR of type: {}\n{}",
                    type_,
                    str::from_utf8(&buf).unwrap_or("Unknown error")
                ));
            }
        } else {
            gl::GetProgramiv(obj, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                // Query info log length
                let mut len: i32 = 0;
                gl::GetProgramiv(obj, gl::INFO_LOG_LENGTH, &mut len);
                let buf_len = if len > 0 { len as usize } else { 1 };
                let mut buf: Vec<u8> = vec![0u8; buf_len];
                gl::GetProgramInfoLog(obj, len, ptr::null_mut(), buf.as_mut_ptr() as *mut i8);
                return Err(format!(
                    "ERROR::PROGRAM_LINKING_ERROR of type: {}\n{}",
                    type_,
                    str::from_utf8(&buf).unwrap_or("Unknown error")
                ));
            }
        }
    }
    Ok(())
}
