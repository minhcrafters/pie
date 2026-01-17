// pie/src/renderer/mesh.rs
use crate::renderer::texture::Texture;
use gl;
use glam::{Vec2, Vec3};
use pyo3::Python;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::mem;
use std::ptr;
use tobj;

#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coords: Vec2,
}

#[pyclass(unsendable)]
pub struct Mesh {
    pub vao: u32,
    pub vbo: u32,
    pub ebo: u32,
    pub index_count: i32,

    /// Optional per-mesh solid color (r,g,b,a) channels 0-255.
    /// When present, renderer uses this color mapping instead of sampling GPU textures.
    #[pyo3(get)]
    pub color: Option<(u8, u8, u8, u8)>,
}

#[pymethods]
impl Mesh {
    /// Attach a `pie.texture.Texture` Python object to this mesh.
    /// Extract and store the color channels so rendering uses per-mesh color mapping.
    /// This preserves the Python-facing API `set_texture(...)` but internally stores a color.
    pub fn set_texture(&mut self, texture: Py<Texture>) {
        Python::attach(|py| {
            let obj = texture.as_ref();
            // Try to read color channels (r,g,b,a) from the Python texture object.
            let r = obj
                .getattr(py, "r")
                .ok()
                .and_then(|v| v.extract::<u8>(py).ok());
            let g = obj
                .getattr(py, "g")
                .ok()
                .and_then(|v| v.extract::<u8>(py).ok());
            let b = obj
                .getattr(py, "b")
                .ok()
                .and_then(|v| v.extract::<u8>(py).ok());
            let a = obj
                .getattr(py, "a")
                .ok()
                .and_then(|v| v.extract::<u8>(py).ok());
            if let (Some(r), Some(g), Some(b), Some(a)) = (r, g, b, a) {
                self.color = Some((r, g, b, a));
            }
        });
    }

    /// Remove any attached color (legacy API name kept for compatibility).
    pub fn clear_texture(&mut self) {
        self.color = None;
    }

    /// Load a mesh from an OBJ file. Exposed to Python.
    #[staticmethod]
    pub fn from_obj(file_path: &str) -> Self {
        match tobj::load_obj(
            file_path,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        ) {
            Ok((models, _materials)) => {
                let mut vertices: Vec<Vertex> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();

                for model in models {
                    let mesh = model.mesh;

                    let positions = &mesh.positions;
                    let normals = &mesh.normals;
                    let texcoords = &mesh.texcoords;

                    for &idx in &mesh.indices {
                        let i = idx as usize;
                        let px = *positions.get(i * 3).unwrap_or(&0.0);
                        let py = *positions.get(i * 3 + 1).unwrap_or(&0.0);
                        let pz = *positions.get(i * 3 + 2).unwrap_or(&0.0);

                        let nx = *normals.get(i * 3).unwrap_or(&0.0);
                        let ny = *normals.get(i * 3 + 1).unwrap_or(&0.0);
                        let nz = *normals.get(i * 3 + 2).unwrap_or(&0.0);

                        let u = *texcoords.get(i * 2).unwrap_or(&0.0);
                        let v = *texcoords.get(i * 2 + 1).unwrap_or(&0.0);

                        vertices.push(Vertex {
                            position: Vec3::new(px, py, pz),
                            normal: Vec3::new(nx, ny, nz),
                            tex_coords: Vec2::new(u, v),
                        });
                        indices.push((vertices.len() - 1) as u32);
                    }
                }

                Mesh::new(vertices, indices)
            }
            Err(_) => Mesh::new(vec![], vec![]),
        }
    }

    /// Create a simple cube mesh (exposed to Python).
    #[staticmethod]
    pub fn cube() -> Self {
        // Cube data (positions/normals/uvs)
        let pos = [
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(-0.5, 0.5, 0.5), // Front
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(0.5, 0.5, 0.5), // Right
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(-0.5, 0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5), // Back
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(-0.5, 0.5, 0.5),
            Vec3::new(-0.5, 0.5, -0.5), // Left
            Vec3::new(-0.5, 0.5, 0.5),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(-0.5, 0.5, -0.5), // Top
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(-0.5, -0.5, 0.5), // Bottom
        ];
        let normals = [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        ];
        let uvs = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for i in 0..6 {
            for j in 0..4 {
                vertices.push(Vertex {
                    position: pos[i * 4 + j],
                    normal: normals[i],
                    tex_coords: uvs[j],
                });
            }
            let base = (i * 4) as u32;
            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base + 2);
            indices.push(base + 3);
            indices.push(base);
        }

        Mesh::new(vertices, indices)
    }

    /// Create an icosphere (exposed to Python).
    #[staticmethod]
    pub fn icosphere(subdivisions: u32) -> Self {
        // Build an icosahedron and subdivide faces `subdivisions` times.
        let t = (1.0f32 + 5.0f32.sqrt()) / 2.0f32;
        let mut positions: Vec<Vec3> = vec![
            Vec3::new(-1.0, t, 0.0),
            Vec3::new(1.0, t, 0.0),
            Vec3::new(-1.0, -t, 0.0),
            Vec3::new(1.0, -t, 0.0),
            Vec3::new(0.0, -1.0, t),
            Vec3::new(0.0, 1.0, t),
            Vec3::new(0.0, -1.0, -t),
            Vec3::new(0.0, 1.0, -t),
            Vec3::new(t, 0.0, -1.0),
            Vec3::new(t, 0.0, 1.0),
            Vec3::new(-t, 0.0, -1.0),
            Vec3::new(-t, 0.0, 1.0),
        ];

        // Normalize initial positions
        for p in positions.iter_mut() {
            *p = p.normalize();
        }

        let mut faces: Vec<[usize; 3]> = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];

        // Helper to get / create midpoint vertex
        let mut midpoint_cache: HashMap<(usize, usize), usize> = HashMap::new();
        let mut get_midpoint = |a: usize, b: usize, positions: &mut Vec<Vec3>| -> usize {
            let (min_i, max_i) = if a < b { (a, b) } else { (b, a) };
            let key = (min_i, max_i);
            if let Some(&idx) = midpoint_cache.get(&key) {
                return idx;
            }
            let pa = positions[a];
            let pb = positions[b];
            let mut mid = (pa + pb) * 0.5;
            mid = mid.normalize();
            let new_idx = positions.len();
            positions.push(mid);
            midpoint_cache.insert(key, new_idx);
            new_idx
        };

        for _ in 0..subdivisions {
            let mut new_faces: Vec<[usize; 3]> = Vec::new();
            for tri in faces.iter() {
                let a = tri[0];
                let b = tri[1];
                let c = tri[2];

                let ab = get_midpoint(a, b, &mut positions);
                let bc = get_midpoint(b, c, &mut positions);
                let ca = get_midpoint(c, a, &mut positions);

                new_faces.push([a, ab, ca]);
                new_faces.push([b, bc, ab]);
                new_faces.push([c, ca, bc]);
                new_faces.push([ab, bc, ca]);
            }
            faces = new_faces;
        }

        // Convert positions and faces into Vertex and index lists. Scale to radius 0.5.
        let radius = 0.5f32;
        let mut verts: Vec<Vertex> = Vec::with_capacity(positions.len());
        for p in positions.iter() {
            let normal = p.normalize();
            let position = *p * radius;
            verts.push(Vertex {
                position,
                normal,
                tex_coords: Vec2::new(0.0, 0.0),
            });
        }
        let mut inds: Vec<u32> = Vec::with_capacity(faces.len() * 3);
        for tri in faces.iter() {
            inds.push(tri[0] as u32);
            inds.push(tri[1] as u32);
            inds.push(tri[2] as u32);
        }

        Mesh::new(verts, inds)
    }

    /// Empty mesh (exposed to Python).
    #[staticmethod]
    pub fn empty() -> Self {
        Mesh::new(vec![], vec![])
    }

    /// Plane primitive (exposed to Python).
    #[staticmethod]
    pub fn plane() -> Self {
        let vertices = vec![
            Vertex {
                position: Vec3::new(-0.5, 0.0, -0.5),
                normal: Vec3::new(0.0, 1.0, 0.0),
                tex_coords: Vec2::new(0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(0.5, 0.0, -0.5),
                normal: Vec3::new(0.0, 1.0, 0.0),
                tex_coords: Vec2::new(1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(0.5, 0.0, 0.5),
                normal: Vec3::new(0.0, 1.0, 0.0),
                tex_coords: Vec2::new(1.0, 1.0),
            },
            Vertex {
                position: Vec3::new(-0.5, 0.0, 0.5),
                normal: Vec3::new(0.0, 1.0, 0.0),
                tex_coords: Vec2::new(0.0, 1.0),
            },
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];
        Mesh::new(vertices, indices)
    }
}

impl Mesh {
    pub fn new(mut vertices: Vec<Vertex>, indices: Vec<u32>) -> Mesh {
        let mut vao = 0;
        let mut vbo = 0;
        let mut ebo = 0;

        // If normals are all-zero, generate normals from the index/triangle data
        if !vertices.is_empty() && vertices.iter().all(|v| v.normal.length_squared() == 0.0) {
            let mut accu = vec![Vec3::ZERO; vertices.len()];
            for tri in indices.chunks(3) {
                if tri.len() < 3 {
                    continue;
                }
                let i0 = tri[0] as usize;
                let i1 = tri[1] as usize;
                let i2 = tri[2] as usize;
                if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
                    continue;
                }
                let v0 = vertices[i0].position;
                let v1 = vertices[i1].position;
                let v2 = vertices[i2].position;
                let face_normal = (v1 - v0).cross(v2 - v0);
                let face_normal = if face_normal.length_squared() != 0.0 {
                    face_normal.normalize()
                } else {
                    Vec3::ZERO
                };
                accu[i0] += face_normal;
                accu[i1] += face_normal;
                accu[i2] += face_normal;
            }
            for (v, n) in vertices.iter_mut().zip(accu.iter()) {
                v.normal = if n.length_squared() != 0.0 {
                    n.normalize()
                } else {
                    *n
                };
            }
        }

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);

            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            let vertex_data_ptr = if vertices.is_empty() {
                ptr::null()
            } else {
                &vertices[0] as *const Vertex as *const _
            };
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<Vertex>()) as isize,
                vertex_data_ptr,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            let index_data_ptr = if indices.is_empty() {
                ptr::null()
            } else {
                &indices[0] as *const u32 as *const _
            };
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<u32>()) as isize,
                index_data_ptr,
                gl::STATIC_DRAW,
            );

            // Vertex Positions
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                ptr::null(),
            );

            // Vertex Normals
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                (mem::size_of::<Vec3>()) as *const _,
            );

            // Vertex Texture Coords
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                (mem::size_of::<Vec3>() * 2) as *const _,
            );

            gl::BindVertexArray(0);
        }

        Mesh {
            vao,
            vbo,
            ebo,
            index_count: indices.len() as i32,
            color: None,
        }
    }

    /// Draw the mesh. Texture mapping has been removed; meshes are rendered using the
    /// currently active shader and any per-mesh color provided via uniforms.
    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawElements(
                gl::TRIANGLES,
                self.index_count,
                gl::UNSIGNED_INT,
                ptr::null(),
            );
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.ebo);
        }
    }
}
