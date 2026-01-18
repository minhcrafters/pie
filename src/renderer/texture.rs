use pyo3::prelude::*;
use std::path::Path;

#[pyclass(unsendable)]
pub struct Texture {
    #[pyo3(get)]
    pub id: u32,

    #[pyo3(get)]
    pub r: u8,
    #[pyo3(get)]
    pub g: u8,
    #[pyo3(get)]
    pub b: u8,
    #[pyo3(get)]
    pub a: u8,
}

#[pymethods]
impl Texture {
    #[staticmethod]
    pub fn from_color(r: u8, g: u8, b: u8, a: u8) -> Self {
        Texture { id: 0, r, g, b, a }
    }

    #[staticmethod]
    pub fn from_image(path: &str) -> Self {
        match load_texture_from_file(path) {
            Ok(id) => Texture {
                id,
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            Err(e) => {
                eprintln!("Failed to load texture '{}': {}", path, e);
                Texture {
                    id: 0,
                    r: 255,
                    g: 0,
                    b: 255,
                    a: 255,
                }
            }
        }
    }

    pub fn to_rgba_f32(&self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if self.id != 0 {
            unsafe {
                gl::DeleteTextures(1, &self.id);
            }
        }
    }
}

fn load_texture_from_file(path: &str) -> Result<u32, String> {
    let img = image::open(Path::new(path)).map_err(|e| format!("Failed to open image: {}", e))?;

    let img = img.to_rgba8();
    let (width, height) = img.dimensions();
    let data = img.into_raw();

    let mut texture_id = 0;
    unsafe {
        gl::GenTextures(1, &mut texture_id);
        gl::BindTexture(gl::TEXTURE_2D, texture_id);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            data.as_ptr() as *const _,
        );

        gl::GenerateMipmap(gl::TEXTURE_2D);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    Ok(texture_id)
}

pub fn create_white_texture() -> u32 {
    let white_pixel: [u8; 4] = [255, 255, 255, 255];

    let mut texture_id = 0;
    unsafe {
        gl::GenTextures(1, &mut texture_id);
        gl::BindTexture(gl::TEXTURE_2D, texture_id);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            1,
            1,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            white_pixel.as_ptr() as *const _,
        );

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    texture_id
}
