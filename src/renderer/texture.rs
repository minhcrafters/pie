// pie/src/renderer/texture.rs
use pyo3::prelude::*;

/// Color-only Texture object exposed to Python.
///
/// This type no longer creates or manages GPU textures. Instead it represents a
/// solid RGBA color (0-255 per channel). The renderer's sampling-based texture
/// path has been removed; by not exposing a GPU `id` attribute here, existing
/// mesh binding code that attempts to read a texture `id` will simply not find one
/// and thus won't enable texture sampling. This enforces color-only (no texture mapping).
#[pyclass(unsendable)]
pub struct Texture {
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
    /// Create a color "texture" from RGBA channels (0-255).
    ///
    /// Example: `Texture.from_color(255, 0, 0, 255)` creates an opaque red color.
    #[staticmethod]
    pub fn from_color(r: u8, g: u8, b: u8, a: u8) -> Self {
        Texture { r, g, b, a }
    }

    /// Legacy shim: image loading is a no-op in color-only mode and returns a magenta
    /// debug color to indicate a missing/invalid resource if the caller expected an image.
    #[staticmethod]
    pub fn from_image(_path: &str) -> Self {
        // Return magenta debug color (clearly visible)
        Texture {
            r: 255,
            g: 0,
            b: 255,
            a: 255,
        }
    }

    /// Return color channels as normalized floats (0.0 - 1.0).
    pub fn to_rgba_f32(&self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }
}
