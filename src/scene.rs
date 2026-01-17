use crate::renderer::mesh::Mesh;
use glam::{EulerRot, Mat4, Quat, Vec3};
use pyo3::prelude::*;

#[derive(Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Transform {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::new()
    }
}

impl Transform {
    pub fn get_model_matrix(&self) -> Mat4 {
        let scale = self.scale;
        let rotation = self.rotation;
        let position = self.position;

        Mat4::from_scale_rotation_translation(scale, rotation, position)
    }
}

#[pyclass]
pub struct Entity {
    pub transform: Transform,
    #[pyo3(get)]
    pub mesh: Option<Py<Mesh>>,
}

#[pymethods]
impl Entity {
    #[new]
    pub fn new() -> Self {
        Entity {
            transform: Transform::new(),
            mesh: None,
        }
    }

    pub fn set_mesh(&mut self, mesh: Py<Mesh>) {
        self.mesh = Some(mesh);
    }

    #[getter]
    pub fn get_position(&self) -> (f32, f32, f32) {
        (
            self.transform.position.x,
            self.transform.position.y,
            self.transform.position.z,
        )
    }

    #[setter]
    pub fn set_position(&mut self, position: (f32, f32, f32)) {
        self.transform.position = Vec3::new(position.0, position.1, position.2);
    }

    #[getter]
    pub fn get_rotation(&self) -> (f32, f32, f32) {
        let (roll, pitch, yaw) = self.transform.rotation.to_euler(EulerRot::XYZ);
        (roll, pitch, yaw)
    }

    #[setter]
    pub fn set_rotation(&mut self, rotation: (f32, f32, f32)) {
        self.transform.rotation =
            Quat::from_euler(EulerRot::XYZ, rotation.0, rotation.1, rotation.2);
    }

    #[getter]
    pub fn get_scale(&self) -> (f32, f32, f32) {
        (
            self.transform.scale.x,
            self.transform.scale.y,
            self.transform.scale.z,
        )
    }

    #[setter]
    pub fn set_scale(&mut self, scale: (f32, f32, f32)) {
        self.transform.scale = Vec3::new(scale.0, scale.1, scale.2);
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}

#[pyclass]
pub struct Camera {
    pub position: Vec3,
    pub front: Vec3,
    pub up: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
}

#[pymethods]
impl Camera {
    #[new]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Camera {
            position: Vec3::new(x, y, z),
            front: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            yaw: -90.0,
            pitch: 0.0,
            fov: 45.0,
        }
    }

    #[getter]
    pub fn get_position(&self) -> (f32, f32, f32) {
        (self.position.x, self.position.y, self.position.z)
    }

    #[setter]
    pub fn set_position(&mut self, position: (f32, f32, f32)) {
        self.position = Vec3::new(position.0, position.1, position.2);
    }

    #[getter]
    pub fn get_fov(&self) -> f32 {
        self.fov
    }

    #[setter]
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
    }

    #[getter]
    pub fn get_yaw_pitch(&self) -> (f32, f32) {
        (self.yaw, self.pitch)
    }

    #[setter]
    pub fn set_yaw_pitch(&mut self, yaw_pitch: (f32, f32)) {
        self.yaw = yaw_pitch.0;
        self.pitch = yaw_pitch.1;

        let front_x = self.yaw.to_radians().cos() * self.pitch.to_radians().cos();
        let front_y = self.pitch.to_radians().sin();
        let front_z = self.yaw.to_radians().sin() * self.pitch.to_radians().cos();
        self.front = Vec3::new(front_x, front_y, front_z).normalize();
    }
}

impl Camera {
    pub fn get_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.front, self.up)
    }
}

#[pyclass]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightType {
    Point = 0,
    Directional = 1,
}

#[pyclass]
pub struct Scene {
    #[pyo3(get)]
    pub entities: Vec<Py<Entity>>,
    #[pyo3(get)]
    pub lights: Vec<Py<Light>>,
}

#[pymethods]
impl Scene {
    #[new]
    pub fn new() -> Self {
        Scene {
            entities: Vec::new(),
            lights: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, entity: Py<Entity>) {
        self.entities.push(entity);
    }

    pub fn add_light(&mut self, light: Py<Light>) {
        self.lights.push(light);
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Light {
    pub position: Vec3,
    pub color: Vec3,
    #[pyo3(get, set)]
    pub radius: f32,
    #[pyo3(get, set)]
    pub light_type: LightType,
}

#[pymethods]
impl Light {
    #[new]
    pub fn new(r: f32, g: f32, b: f32, radius: f32, light_type: LightType) -> Self {
        Light {
            position: Vec3::new(0.0, 0.0, 0.0),
            color: Vec3::new(r, g, b),
            radius,
            light_type,
        }
    }

    #[staticmethod]
    pub fn point(r: f32, g: f32, b: f32, radius: f32) -> Self {
        Light {
            position: Vec3::new(0.0, 0.0, 0.0),
            color: Vec3::new(r, g, b),
            radius,
            light_type: LightType::Point,
        }
    }

    #[staticmethod]
    pub fn directional(r: f32, g: f32, b: f32) -> Self {
        // Directional lights don't use radius.
        Light {
            position: Vec3::new(0.0, 0.0, 0.0), // For directional, this represents direction
            color: Vec3::new(r, g, b),
            radius: 0.0, // Not used for directional lights
            light_type: LightType::Directional,
        }
    }

    // Radius is exposed via #[pyo3(get, set)] on the struct; no custom getter/setter required here.

    #[getter]
    pub fn get_position(&self) -> (f32, f32, f32) {
        (self.position.x, self.position.y, self.position.z)
    }

    #[setter]
    pub fn set_position(&mut self, position: (f32, f32, f32)) {
        self.position = Vec3::new(position.0, position.1, position.2);
    }

    #[getter]
    pub fn get_color(&self) -> (f32, f32, f32) {
        (self.color.x, self.color.y, self.color.z)
    }

    #[setter]
    pub fn set_color(&mut self, color: (f32, f32, f32)) {
        self.color = Vec3::new(color.0, color.1, color.2);
    }
}
