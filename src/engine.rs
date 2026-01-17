use glam::{Mat4, Quat, Vec3};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sdl2::Sdl;
use sdl2::video::GLContext;
use sdl2::video::Window;

use crate::input::InputState;
use crate::physics::PhysicsWorld;
use crate::renderer::Renderer;
use crate::scene::LightType;
use crate::scene::{Camera, Entity, Light, Scene};

use crate::audio::{AudioMixer, AudioSource, ListenerState};
use std::sync::{Arc, Mutex};

/// Wrapper for exposing SDL2 events to Python.
///
/// We expose a compact, but descriptive Python-visible structure that contains:
///  - `kind`: a textual debug representation of the full SDL event
///  - `timestamp`: the event timestamp (if available; otherwise 0)
///  - `name`: a short event variant name (same as `kind` here, but separated for convenience)
///  - `details`: a detailed debug representation (multi-line) useful for introspection
///
/// The Rust side will create `Py<SdlEvent>` objects and push them into the
/// engine's pending queue. Python code can call `take_pending_events()` to
/// drain the queue and inspect events.
#[pyclass]
pub struct SdlEvent {
    #[pyo3(get)]
    pub timestamp: u32,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub details: String,
    #[pyo3(get)]
    pub fields: Py<PyDict>,
}

#[pyclass(unsendable)]
pub struct Engine {
    sdl_context: Sdl,
    _video_subsystem: sdl2::VideoSubsystem,
    _audio_subsystem: sdl2::AudioSubsystem,
    _audio_device: sdl2::audio::AudioDevice<AudioMixer>,
    window: Option<Window>,
    _gl_context: GLContext,

    renderer: Renderer,
    scene: Scene,
    camera: Py<Camera>,
    physics_world: PhysicsWorld,
    input: InputState,

    should_quit: bool,

    audio_sources: Arc<Mutex<Vec<Py<AudioSource>>>>,
    /// Pending non-input SDL events exposed to Python as `Py<SdlEvent>`.
    /// Use `take_pending_events()` to fetch and clear the queue on the Python side.
    pending_events: Arc<Mutex<Vec<Py<SdlEvent>>>>,
    listener_state: Arc<Mutex<ListenerState>>,
}

#[pymethods]
impl Engine {
    #[new]
    pub fn new(title: &str, width: u32, height: u32) -> PyResult<Self> {
        let sdl_context =
            sdl2::init().map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;
        let video_subsystem = sdl_context
            .video()
            .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;
        let audio_subsystem = sdl_context
            .audio()
            .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;

        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 3);

        let window = video_subsystem
            .window(title, width, height)
            .opengl()
            .resizable()
            .position_centered()
            .build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let _gl_context = window
            .gl_create_context()
            .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;

        gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const _);

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
        }

        let renderer = Renderer::new(width, height)
            .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;
        let scene = Scene::new();
        let camera = Python::attach(|py| {
            Py::new(py, Camera::new(0.0, 0.0, 0.0))
                .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
                .unwrap()
        });
        let physics_world = PhysicsWorld::new();
        let input_state = InputState::new();

        // Audio Setup
        let desired_spec = sdl2::audio::AudioSpecDesired {
            freq: Some(44100),
            channels: Some(2),
            samples: None,
        };

        let audio_sources = Arc::new(Mutex::new(Vec::new()));
        let initial_sources = audio_sources.clone();

        let listener_state = Arc::new(Mutex::new(ListenerState {
            position: Vec3::ZERO,
            right: Vec3::X,
        }));
        let listener_state_clone = listener_state.clone();

        // Recording infrastructure intentionally not exposed; no local recorder handle required.

        // Queue of pending SDL events to expose to Python. We store `Py<SdlEvent>`
        // objects here so the Python side receives proper Python classes.
        let pending_events = Arc::new(Mutex::new(Vec::new()));
        let pending_events_clone = pending_events.clone();

        let device = audio_subsystem
            .open_playback(None, &desired_spec, |_spec| {
                AudioMixer::new(initial_sources.clone(), listener_state_clone.clone())
            })
            .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;

        device.resume();

        // Construct engine and set default for graceful quit flag
        let engine = Engine {
            sdl_context,
            _video_subsystem: video_subsystem,
            _audio_subsystem: audio_subsystem,
            _audio_device: device,
            window: Some(window),
            _gl_context,
            renderer,
            scene,
            camera,
            physics_world,
            input: input_state,
            should_quit: false,
            audio_sources,
            pending_events: pending_events_clone,
            listener_state,
        };

        Ok(engine)
    }

    #[getter]
    pub fn get_camera(&mut self, py: Python) -> Py<Camera> {
        self.camera.clone_ref(py)
    }

    #[setter]
    pub fn set_camera(&mut self, camera: Py<Camera>) {
        self.camera = camera;
    }

    pub fn add_entity(&mut self, entity: Py<Entity>) {
        self.scene.add_entity(entity);
    }

    pub fn add_light(&mut self, light: Py<Light>) {
        self.scene.add_light(light);
    }

    pub fn add_audio_source(&mut self, source: Py<AudioSource>) {
        let mut sources = self.audio_sources.lock().unwrap();
        sources.push(source);
    }

    pub fn configure_point_lights(&mut self, num_point_lights: usize) {
        self.renderer.configure_point_lights(num_point_lights);
    }

    pub fn is_key_down(&self, key_name: &str) -> bool {
        self.input.is_key_down(key_name)
    }

    pub fn is_mouse_down(&self, button: &str) -> bool {
        match button {
            "Left" => self.input.is_mouse_down(sdl2::mouse::MouseButton::Left),
            "Right" => self.input.is_mouse_down(sdl2::mouse::MouseButton::Right),
            "Middle" => self.input.is_mouse_down(sdl2::mouse::MouseButton::Middle),
            _ => false,
        }
    }

    pub fn get_mouse_pos(&self) -> (i32, i32) {
        (self.input.mouse_pos.x as i32, self.input.mouse_pos.y as i32)
    }

    pub fn get_mouse_rel(&self) -> (i32, i32) {
        (self.input.mouse_rel.x as i32, self.input.mouse_rel.y as i32)
    }

    pub fn set_mouse_capture(&mut self, enabled: bool) {
        self.input.mouse_captured = enabled;
        self.sdl_context.mouse().set_relative_mouse_mode(enabled);

        if enabled && let Some(win) = &self.window {
            let (w, h) = win.size();
            self.sdl_context
                .mouse()
                .warp_mouse_in_window(win, (w / 2) as i32, (h / 2) as i32);
        }
    }

    /// Request a graceful quit of the engine.
    /// This signals the engine to stop; on the next call `update()` will return `false`
    /// allowing the Python-side main loop to exit cleanly.
    pub fn quit(&mut self) {
        self.stop_peripherals();
        self.should_quit = true;
    }

    pub fn update(&mut self) -> PyResult<bool> {
        self.input.prepare_update();

        let mut event_pump = self
            .sdl_context
            .event_pump()
            .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;

        for event in event_pump.poll_iter() {
            self.input.process_event(&event);

            if let sdl2::event::Event::Window {
                win_event:
                    sdl2::event::WindowEvent::Resized(w, h)
                    | sdl2::event::WindowEvent::SizeChanged(w, h),
                ..
            } = event
            {
                let width = if w > 0 { w } else { 1 };
                let height = if h > 0 { h } else { 1 };

                self.renderer.resize(width as u32, height as u32);

                unsafe {
                    gl::Viewport(0, 0, width, height);
                }
            }

            Python::attach(|py| {
                let full = format!("{:?}", event);
                let name = if let Some(idx) = full.find('{') {
                    full[..idx].trim().to_string()
                } else {
                    full.clone()
                };
                let details = format!("{:#?}", event);
                let dict = PyDict::new(py);
                let _ = dict.set_item("raw", details.clone());
                let _ = dict.set_item("variant", name.clone());

                let ts = event.get_timestamp();
                let _ = dict.set_item("timestamp", ts);

                if let Ok(py_ev) = Py::new(
                    py,
                    SdlEvent {
                        name: name.clone(),
                        timestamp: ts,
                        details: details.clone(),
                        fields: dict.unbind(),
                    },
                ) && let Ok(mut q) = self.pending_events.lock()
                {
                    q.push(py_ev);
                }
            });
        }

        // Check for quit after processing events - if set by other code, exit the update loop
        if self.should_quit {
            return Ok(false);
        }

        self.physics_world.step();

        Python::attach(|py| {
            if let Ok(mut state) = self.listener_state.lock()
                && let Ok(camera) = self.camera.try_borrow_mut(py)
            {
                state.position = camera.position;
                state.right = camera.front.cross(camera.up).normalize_or_zero();
            }

            // println!("Rendering Shadows...");
            self.render_shadows();

            // println!("Rendering Geometry...");
            self.renderer.begin_geometry_pass();

            // Use glam perspective
            let projection = Mat4::perspective_rh_gl(
                self.camera.borrow(py).fov.to_radians(),
                self.renderer.width as f32 / self.renderer.height as f32,
                0.001,
                1000.0,
            );
            let view = self.camera.borrow(py).get_view_matrix();

            let shader = self.renderer.get_geometry_shader();
            shader.use_program();
            shader.set_mat4("view", &view);
            shader.set_mat4("projection", &projection);

            for entity_py in &self.scene.entities {
                let entity = entity_py.borrow(py);
                if let Some(mesh) = &entity.mesh {
                    let mesh_ref = mesh.borrow(py);
                    // Set model matrix for this entity
                    shader.set_mat4("model", &entity.transform.get_model_matrix());
                    // Determine per-mesh albedo color (rgb) and specular intensity (a).
                    // Mesh.color is Option<(u8,u8,u8,u8)> where channels are 0-255.
                    if let Some((r, g, b, a)) = mesh_ref.color {
                        let rf = r as f32 / 255.0;
                        let gf = g as f32 / 255.0;
                        let bf = b as f32 / 255.0;
                        let af = a as f32 / 255.0;
                        // Set the geometry shader's `albedoColor` uniform (vec4)
                        unsafe {
                            let loc = gl::GetUniformLocation(
                                shader.id,
                                std::ffi::CString::new("albedoColor").unwrap().as_ptr(),
                            );
                            gl::Uniform4f(loc, rf, gf, bf, af);
                        }
                    } else {
                        // Default material color (fallback)
                        unsafe {
                            let loc = gl::GetUniformLocation(
                                shader.id,
                                std::ffi::CString::new("albedoColor").unwrap().as_ptr(),
                            );
                            gl::Uniform4f(loc, 0.95, 0.95, 0.95, 0.5);
                        }
                    }
                    mesh_ref.draw();
                }
            }

            self.renderer.end_geometry_pass();

            self.renderer.begin_lighting_pass();

            // Pass lights to shader
            let shader = &self.renderer.lighting_shader;
            shader.use_program();
            shader.set_vec3("viewPos", &self.camera.borrow(py).position);

            let lights = &self.scene.lights;
            shader.set_int("numLights", lights.len() as i32);

            let mut directional_light_index = None;
            let mut first_point_light_index = None;

            // Find first directional and first point light indices
            for (i, light_py) in lights.iter().enumerate() {
                let light = light_py.borrow(py);
                if light.light_type == LightType::Directional && directional_light_index.is_none() {
                    directional_light_index = Some(i);
                } else if light.light_type == LightType::Point && first_point_light_index.is_none()
                {
                    first_point_light_index = Some(i);
                }
            }

            let mut point_light_count = 0;
            for (i, light_py) in lights.iter().enumerate() {
                let light = light_py.borrow(py);
                let name_pos = format!("lights[{}].Position", i);
                let name_col = format!("lights[{}].Color", i);
                let name_lin = format!("lights[{}].Linear", i);
                let name_quad = format!("lights[{}].Quadratic", i);
                let name_rad = format!("lights[{}].Radius", i);
                let name_type = format!("lights[{}].Type", i);
                let name_has_shadow = format!("lights[{}].HasShadow", i);

                shader.set_vec3(&name_pos, &light.position);
                shader.set_vec3(&name_col, &light.color);
                shader.set_int(&name_type, light.light_type as i32);

                let linear = 4.5 / light.radius;
                let quadratic = 75.0 / (light.radius * light.radius);

                shader.set_float(&name_lin, linear);
                shader.set_float(&name_quad, quadratic);
                shader.set_float(&name_rad, light.radius);

                // Set shadow flags - all directional and point lights can have shadows
                let has_shadow = if light.light_type == LightType::Directional
                    || light.light_type == LightType::Point
                {
                    1
                } else {
                    0
                };
                shader.set_int(&name_has_shadow, has_shadow);

                // Set shadow map index for point lights
                let shadow_map_index = if light.light_type == LightType::Point {
                    let index = point_light_count;
                    point_light_count += 1;
                    index
                } else {
                    0 // Not used for directional lights
                };
                let name_shadow_index = format!("lights[{}].ShadowMapIndex", i);
                shader.set_int(&name_shadow_index, shadow_map_index);

                if light.light_type == LightType::Directional {
                    // Treat `light.position` as a direction vector. Compute a world-space
                    // transform for the directional light and pass the direction to the
                    // lighting shader using the `directionalLightDir` uniform. Also set
                    // the directional light's shadow matrix under the `directionalLightSpaceMatrix`
                    // uniform so the lighting shader can sample the shadow map.
                    let dir = if light.position.length_squared() > 0.000001 {
                        light.position.normalize()
                    } else {
                        Vec3::new(0.0, -1.0, 0.0)
                    };
                    // We want `directionalLightDir` to point from the fragment toward the light,
                    // so negate the stored direction (stored as `light.position`) here.
                    let directional_light_dir = -dir;
                    let light_pos = directional_light_dir * 30.0;
                    let light_projection =
                        Mat4::orthographic_rh_gl(-10.0, 10.0, -10.0, 10.0, 1.0, 7.5);
                    let light_view = Mat4::look_at_rh(light_pos, Vec3::ZERO, Vec3::Y);
                    let light_space_matrix = light_projection * light_view;
                    // Pass both the light-space matrix and the direction into the lighting shader.
                    shader.set_mat4("directionalLightSpaceMatrix", &light_space_matrix);
                    shader.set_vec3("directionalLightDir", &directional_light_dir);
                    // Bind directional shadow map to the texture unit expected by the shader (3).
                    unsafe {
                        gl::ActiveTexture(gl::TEXTURE3);
                        gl::BindTexture(gl::TEXTURE_2D, self.renderer.directional_shadow_map);
                    }
                } else if light.light_type == LightType::Point {
                    shader.set_float("farPlane", 25.0);
                }
            }

            // Render lighting pass into HDR color (fullscreen quad)
            self.renderer.render_quad();

            // Copy depth from geometry-pass (G-buffer) into HDR FBO so spheres depth-test correctly.
            self.renderer.blit_depth_from_gbuffer_to_hdr();

            // Render unshaded sphere meshes for each point light (visualization + contribute to HDR/bloom)
            // This runs while HDR framebuffer is bound (we called begin_lighting_pass before entering this closure),
            // and uses the view/projection matrices already available in this scope.
            for light_py in &self.scene.lights {
                let light = light_py.borrow(py);
                if light.light_type == LightType::Point {
                    // Use the Rust-side `light.radius` field directly so runtime assignments from Python
                    // (via pyo3) update the value that we read here.
                    // Increase visual scale multiplier so sphere size follows radius more noticeably.
                    let visual_scale = light.radius * 0.2;
                    let model = Mat4::from_scale_rotation_translation(
                        Vec3::splat(visual_scale),
                        Quat::IDENTITY,
                        light.position,
                    );

                    self.renderer
                        .render_sphere_at(&model, &view, &projection, &light.color);
                }
            }
        });

        // Extract bright areas for bloom from HDR buffer (bright-pass)
        self.renderer.begin_bloom_extract_pass();
        self.renderer.render_quad();
        self.renderer.end_bloom_extract_pass();

        // Apply separable Gaussian blur to the bright-pass to produce bloom texture.
        // Use N iterations (each iteration is one horizontal or vertical pass).
        // 10 iterations is a common choice (5 horizontal + 5 vertical).
        self.renderer.apply_gaussian_blur(10);

        // Composite (tone mapping + additive bloom)
        self.renderer.begin_composite_pass();
        self.renderer.render_quad();

        if let Some(win) = &self.window {
            win.gl_swap_window();
        }

        Ok(true)
    }

    /// Return and clear any pending non-input SDL events queued for Python.
    /// The returned list contains `SdlEvent` Python objects.
    pub fn poll_events(&mut self) -> PyResult<Vec<Py<SdlEvent>>> {
        if let Ok(mut q) = self.pending_events.lock() {
            let events: Vec<Py<SdlEvent>> = q.drain(..).collect();
            Ok(events)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn move_camera(&mut self, dx: f32, dy: f32, dz: f32) {
        Python::attach(|py| {
            self.camera.borrow_mut(py).position.x += dx;
            self.camera.borrow_mut(py).position.y += dy;
            self.camera.borrow_mut(py).position.z += dz;
        });
    }

    pub fn rotate_camera(&mut self, yaw: f32, pitch: f32) {
        Python::attach(|py| {
            let mut camera = self.camera.borrow_mut(py);

            camera.yaw += yaw;
            camera.pitch += pitch;

            camera.pitch = camera.pitch.clamp(-89.0, 89.0);

            let front_x = camera.yaw.to_radians().cos() * camera.pitch.to_radians().cos();
            let front_y = camera.pitch.to_radians().sin();
            let front_z = camera.yaw.to_radians().sin() * camera.pitch.to_radians().cos();
            camera.front = Vec3::new(front_x, front_y, front_z).normalize();
        });
    }
}

impl Engine {
    /// Stop peripherals like audio and release mouse capture. Safe to call multiple times.
    fn stop_peripherals(&mut self) {
        // Pause the audio device so the audio callback stops running
        self._audio_device.pause();

        // Ensure all audio sources stop playing.
        // Acquire the Python GIL before locking the sources mutex to avoid deadlocks
        // that can occur if the audio callback thread holds the mutex while code on the
        // main thread tries to acquire the GIL (or vice-versa). We attach to the Python
        // interpreter first, then lock the mutex while the GIL is held.
        Python::attach(|py| {
            if let Ok(mut sources) = self.audio_sources.lock() {
                for src in sources.iter_mut() {
                    if let Ok(mut s) = src.try_borrow_mut(py) {
                        s.playing = false;
                    }
                }
            }
        });

        // Release any mouse capture
        self.set_mouse_capture(false);

        // Note: other subsystems (GL context, audio subsystem) will be released when
        // the Engine is dropped. Pausing audio and stopping sources
        // prevents lingering activity while the application shuts down.
    }

    fn render_shadows(&mut self) {
        Python::attach(|py| {
            let lights = &self.scene.lights;
            let mut directional_light_index = None;
            let mut first_point_light_index = None;

            // Find first directional and first point light indices
            for (i, light_py) in lights.iter().enumerate() {
                let light = light_py.borrow(py);
                if light.light_type == LightType::Directional && directional_light_index.is_none() {
                    directional_light_index = Some(i);
                } else if light.light_type == LightType::Point && first_point_light_index.is_none()
                {
                    first_point_light_index = Some(i);
                }
            }

            // Render directional shadow map
            if let Some(dir_index) = directional_light_index {
                let light_py = &lights[dir_index];
                let light = light_py.borrow(py);
                // For directional lights, position is actually direction. Calculate light position far away
                let light_direction = -light.position.normalize();
                let light_pos = light_direction * 30.0; // Position light far away in the direction it's coming from

                self.renderer.begin_directional_shadow_pass();

                let light_projection =
                    Mat4::orthographic_rh_gl(-20.0, 20.0, -20.0, 20.0, 1.0, 50.0);
                let light_view = Mat4::look_at_rh(
                    light_pos,
                    Vec3::ZERO, // Look at scene center
                    Vec3::Y,
                );
                let light_space_matrix = light_projection * light_view;

                let shader = &self.renderer.directional_shadow_shader;
                shader.use_program();
                shader.set_mat4("lightSpaceMatrix", &light_space_matrix);

                for entity_py in &self.scene.entities {
                    let entity = entity_py.borrow(py);
                    if let Some(mesh) = &entity.mesh {
                        shader.set_mat4("model", &entity.transform.get_model_matrix());
                        mesh.borrow(py).draw();
                    }
                }

                self.renderer.end_directional_shadow_pass();

                // Pass to lighting shader
                let lighting_shader = &self.renderer.lighting_shader;
                lighting_shader.use_program();
                lighting_shader.set_mat4("directionalLightSpaceMatrix", &light_space_matrix);
                lighting_shader.set_vec3("directionalLightDir", &light_direction);
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE3);
                    gl::BindTexture(gl::TEXTURE_2D, self.renderer.directional_shadow_map);
                }
            }

            // Render point light shadows
            let mut point_light_shadow_index = 0;
            for light_py in lights.iter() {
                let light = light_py.borrow(py);
                if light.light_type == LightType::Point
                    && point_light_shadow_index < self.renderer.point_shadow_maps.len()
                {
                    let far_plane = 25.0;
                    let shadow_transforms =
                        self.get_point_light_transforms(light.position, far_plane);

                    for (face, mat) in shadow_transforms.iter().enumerate() {
                        self.renderer
                            .begin_point_shadow_pass(point_light_shadow_index, face);

                        let shader = &self.renderer.point_shadow_shader;
                        shader.use_program();
                        shader.set_mat4("shadowMatrix", mat);
                        shader.set_vec3("lightPos", &light.position);
                        shader.set_float("farPlane", far_plane);

                        for entity_py in &self.scene.entities {
                            let entity = entity_py.borrow(py);
                            if let Some(mesh) = &entity.mesh {
                                shader.set_mat4("model", &entity.transform.get_model_matrix());
                                mesh.borrow(py).draw();
                            }
                        }

                        self.renderer.end_point_shadow_pass();
                    }

                    // Bind the shadow map to the appropriate texture unit
                    let lighting_shader = &self.renderer.lighting_shader;
                    lighting_shader.use_program();
                    unsafe {
                        gl::ActiveTexture(gl::TEXTURE4 + point_light_shadow_index as u32);
                        gl::BindTexture(
                            gl::TEXTURE_CUBE_MAP,
                            self.renderer.point_shadow_maps[point_light_shadow_index],
                        );
                    }

                    point_light_shadow_index += 1;
                }
            }
        });
    }

    fn get_point_light_transforms(&self, light_pos: Vec3, far_plane: f32) -> [Mat4; 6] {
        let shadow_proj = Mat4::perspective_rh_gl(90.0f32.to_radians(), 1.0, 0.1, far_plane);

        [
            shadow_proj * Mat4::look_at_rh(light_pos, light_pos + Vec3::X, -Vec3::Y),
            shadow_proj * Mat4::look_at_rh(light_pos, light_pos - Vec3::X, -Vec3::Y),
            shadow_proj * Mat4::look_at_rh(light_pos, light_pos + Vec3::Y, Vec3::Z),
            shadow_proj * Mat4::look_at_rh(light_pos, light_pos - Vec3::Y, -Vec3::Z),
            shadow_proj * Mat4::look_at_rh(light_pos, light_pos + Vec3::Z, -Vec3::Y),
            shadow_proj * Mat4::look_at_rh(light_pos, light_pos - Vec3::Z, -Vec3::Y),
        ]
    }
}
