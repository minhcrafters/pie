pub mod mesh;
pub mod shader;
pub mod texture;

use gl;
use shader::Shader;
use std::mem;
use std::ptr;

pub struct Renderer {
    g_buffer: u32,
    g_position: u32,
    g_normal: u32,
    g_albedo_spec: u32,
    rbo_depth: u32,
    hdr_fbo: u32,
    hdr_color: u32,

    // Bloom
    bloom_fbo: u32,
    bloom_color: u32,         // bright-pass capture
    pingpong_fbos: [u32; 2],  // for separable blur
    pingpong_color: [u32; 2], // color attachments for ping-pong
    bloom_output: u32,        // stores the final blurred texture id (one of pingpong_color)
    bloom_shader: Shader,
    blur_shader: Shader,
    pub bloom_enabled: bool,
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,

    // Shadow mapping
    pub directional_shadow_fbo: u32,
    pub directional_shadow_map: u32,
    pub point_shadow_fbos: Vec<[u32; 6]>, // For cube map faces per light
    pub point_shadow_maps: Vec<u32>,      // Cube maps per light

    // Shaders
    pub geometry_shader: Shader,
    pub lighting_shader: Shader,
    pub composite_shader: Shader,
    pub directional_shadow_shader: Shader,
    pub point_shadow_shader: Shader,

    quad_vao: u32,

    // Light-sphere visualization for point lights (unshaded emissive sphere)
    light_sphere: mesh::Mesh,
    light_sphere_shader: Shader,

    pub width: u32,
    pub height: u32,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Result<Renderer, String> {
        let (g_buffer, g_position, g_normal, g_albedo_spec, rbo_depth) =
            unsafe { create_g_buffer(width, height) };
        let (quad_vao, _quad_vbo) = unsafe { create_quad() };
        let (hdr_fbo, hdr_color) = unsafe { create_hdr_buffer(width, height) };
        let (directional_shadow_fbo, directional_shadow_map) =
            unsafe { create_directional_shadow_buffer() };

        let point_shadow_fbos = Vec::new();
        let point_shadow_maps = Vec::new();

        let geometry_shader = Shader::new(
            include_str!("shaders/geometry.vsh"),
            include_str!("shaders/geometry.fsh"),
        )?;
        let lighting_shader = Shader::new(
            include_str!("shaders/lighting.vsh"),
            include_str!("shaders/lighting.fsh"),
        )?;
        let composite_shader = Shader::new(
            include_str!("shaders/composite.vsh"),
            include_str!("shaders/composite.fsh"),
        )?;
        let directional_shadow_shader = Shader::new(
            include_str!("shaders/directional_shadow.vsh"),
            include_str!("shaders/directional_shadow.fsh"),
        )?;
        let point_shadow_shader = Shader::new(
            include_str!("shaders/point_shadow.vsh"),
            include_str!("shaders/point_shadow.fsh"),
        )?;
        let bloom_shader = Shader::new(
            include_str!("shaders/bloom.vsh"),
            include_str!("shaders/bloom.fsh"),
        )?;
        let blur_shader = Shader::new(
            include_str!("shaders/blur.vsh"),
            include_str!("shaders/blur.fsh"),
        )?;
        let light_sphere_shader = Shader::new(
            include_str!("shaders/light_sphere.vsh"),
            include_str!("shaders/light_sphere.fsh"),
        )?;

        let (bloom_fbo, bloom_color) = unsafe { create_bloom_buffer(width, height) };
        let (pingpong_fbos, pingpong_color) = unsafe { create_pingpong_buffers(width, height) };

        geometry_shader.use_program();

        unsafe {
            let loc = gl::GetUniformLocation(
                geometry_shader.id,
                std::ffi::CString::new("albedoColor").unwrap().as_ptr(),
            );
            gl::Uniform4f(loc, 0.95, 0.95, 0.95, 0.5);
        }

        lighting_shader.use_program();
        lighting_shader.set_int("gPosition", 0);
        lighting_shader.set_int("gNormal", 1);
        lighting_shader.set_int("gAlbedoSpec", 2);
        lighting_shader.set_int("directionalShadowMap", 3);

        // Point shadow maps start at texture unit 4
        for i in 0..16 {
            let uniform_name = format!("pointShadowMaps[{}]", i);
            lighting_shader.set_int(&uniform_name, 4 + i);
        }

        composite_shader.use_program();
        composite_shader.set_int("scene", 0);
        composite_shader.set_int("bloomBlur", 1);
        composite_shader.set_int("toneMappingMode", 1);
        composite_shader.set_float("exposure", 1.0);
        composite_shader.set_float("bloomIntensity", 1.0);

        blur_shader.use_program();
        blur_shader.set_int("image", 0);

        bloom_shader.use_program();
        bloom_shader.set_int("scene", 0);
        bloom_shader.set_float("threshold", 0.05);

        let light_sphere = mesh::Mesh::icosphere(2);

        Ok(Renderer {
            g_buffer,
            g_position,
            g_normal,
            g_albedo_spec,
            rbo_depth,
            hdr_fbo,
            hdr_color,
            bloom_fbo,
            bloom_color,
            pingpong_fbos,
            pingpong_color,
            bloom_output: pingpong_color[0],
            bloom_shader,
            blur_shader,
            bloom_enabled: true,
            bloom_threshold: 0.3,
            bloom_intensity: 1.0,
            directional_shadow_fbo,
            directional_shadow_map,
            point_shadow_fbos,
            point_shadow_maps,
            geometry_shader,
            lighting_shader,
            composite_shader,
            directional_shadow_shader,
            point_shadow_shader,
            quad_vao,
            light_sphere,
            light_sphere_shader,
            width,
            height,
        })
    }

    pub fn configure_point_lights(&mut self, num_point_lights: usize) {
        // Clean up existing point shadow resources
        unsafe {
            for fbos in &self.point_shadow_fbos {
                gl::DeleteFramebuffers(6, fbos.as_ptr());
            }
            for &map in &self.point_shadow_maps {
                gl::DeleteTextures(1, &map);
            }
        }

        self.point_shadow_fbos.clear();
        self.point_shadow_maps.clear();

        // Create new point shadow resources
        for _ in 0..num_point_lights {
            let (fbos, map) = unsafe { create_point_shadow_buffer() };
            self.point_shadow_fbos.push(fbos);
            self.point_shadow_maps.push(map);
        }
    }

    pub fn begin_composite_pass(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        self.composite_shader.use_program();
        // composite shader expects scene at 0 and bloomBlur at 1
        self.composite_shader.set_int("scene", 0);
        self.composite_shader.set_int("bloomBlur", 1);
        self.composite_shader.set_int("toneMappingMode", 1);
        self.composite_shader.set_float("exposure", 1.0);
        self.composite_shader
            .set_float("bloomIntensity", self.bloom_intensity);

        unsafe {
            // bind the HDR color buffer as the source texture for tonemapping (unit 0)
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.hdr_color);

            // bind blurred bloom texture as unit 1
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, self.bloom_output);
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        unsafe {
            gl::DeleteFramebuffers(1, &self.g_buffer);
            gl::DeleteTextures(1, &self.g_position);
            gl::DeleteTextures(1, &self.g_normal);
            gl::DeleteTextures(1, &self.g_albedo_spec);
            gl::DeleteRenderbuffers(1, &self.rbo_depth);
            gl::DeleteFramebuffers(1, &self.hdr_fbo);
            gl::DeleteTextures(1, &self.hdr_color);

            // delete bloom resources
            gl::DeleteFramebuffers(1, &self.bloom_fbo);
            gl::DeleteTextures(1, &self.bloom_color);
            gl::DeleteFramebuffers(2, self.pingpong_fbos.as_ptr());
            gl::DeleteTextures(2, self.pingpong_color.as_ptr());

            let (gb, gp, gn, ga, rdo) = create_g_buffer(width, height);
            self.g_buffer = gb;
            self.g_position = gp;
            self.g_normal = gn;
            self.g_albedo_spec = ga;
            self.rbo_depth = rdo;
            // create_hdr_buffer now returns (fbo, hdr_color)
            let (hf, hc) = create_hdr_buffer(width, height);
            self.hdr_fbo = hf;
            self.hdr_color = hc;

            // recreate bloom buffers
            let (bf, bc) = create_bloom_buffer(width, height);
            self.bloom_fbo = bf;
            self.bloom_color = bc;
            let (pp_fbos, pp_cols) = create_pingpong_buffers(width, height);
            self.pingpong_fbos = pp_fbos;
            self.pingpong_color = pp_cols;
            self.bloom_output = self.pingpong_color[0];
        }
    }

    pub fn begin_geometry_pass(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.g_buffer);
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        self.geometry_shader.use_program();
        // Set matrices elsewhere or pass them here
    }

    pub fn get_geometry_shader(&self) -> &Shader {
        &self.geometry_shader
    }

    pub fn end_geometry_pass(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    pub fn begin_lighting_pass(&self) {
        unsafe {
            // Render lighting into HDR framebuffer
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.hdr_fbo);
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            self.lighting_shader.use_program();
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.g_position);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, self.g_normal);
            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, self.g_albedo_spec);
        }
    }

    /// Begin bright-pass extraction into bloom FBO.
    /// Should be called after lighting pass finishes (HDR color contains scene).
    pub fn begin_bloom_extract_pass(&self) {
        if !self.bloom_enabled {
            return;
        }
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.bloom_fbo);
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        self.bloom_shader.use_program();
        // bind the HDR scene color (first HDR attachment) to texture unit 0 so bloom extracts bright areas from the full scene
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.hdr_color);
        }
        // set threshold uniform
        self.bloom_shader
            .set_float("threshold", self.bloom_threshold);
    }

    /// Unbind bloom FBO (returns to default framebuffer).
    pub fn end_bloom_extract_pass(&self) {
        if !self.bloom_enabled {
            return;
        }
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    /// Perform separable Gaussian blur on the bright-pass texture using ping-pong FBOs.
    /// This function updates `bloom_output` to the resulting blurred texture id.
    pub fn apply_gaussian_blur(&mut self, iterations: i32) {
        if !self.bloom_enabled {
            // set output to a 1x1 black texture (or just keep a valid texture)
            self.bloom_output = self.pingpong_color[0];
            return;
        }

        let mut horizontal = true;
        let mut first_iteration = true;
        let mut read_tex: u32;

        self.blur_shader.use_program();

        for _ in 0..iterations {
            let idx = if horizontal { 1 } else { 0 };
            unsafe {
                // Bind the pingpong framebuffer for writing
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.pingpong_fbos[idx]);
                gl::Viewport(0, 0, self.width as i32, self.height as i32);
                // Set the horizontal uniform
            }
            // set horizontal uniform on shader (as int 0/1)
            self.blur_shader
                .set_int("horizontal", if horizontal { 1 } else { 0 });

            unsafe {
                // Bind the correct source texture to texture unit 0
                gl::ActiveTexture(gl::TEXTURE0);
            }
            if first_iteration {
                // source is bloom_color (bright-pass)
                read_tex = self.bloom_color;
                first_iteration = false;
            } else {
                // source toggles between pingpong_color[0/1]
                read_tex = self.pingpong_color[if horizontal { 0 } else { 1 }];
            }

            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, read_tex);
                // Render screen quad
                self.render_quad();
                // Unbind framebuffer after drawing to it
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            }

            horizontal = !horizontal;
        }

        // After finishing, the last written buffer is the opposite of horizontal (since toggled at end)
        let final_idx = if horizontal { 0 } else { 1 };
        self.bloom_output = self.pingpong_color[final_idx];
    }

    pub fn render_quad(&self) {
        unsafe {
            gl::BindVertexArray(self.quad_vao);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::BindVertexArray(0);
        }
    }

    /// Render an unshaded sphere for a point light.
    /// Caller must supply model, view, projection matrices and the light color.
    /// This draws the prebuilt icosphere mesh with a simple unlit shader so the sphere appears as a solid color without lighting.
    /// The sphere will depth-test correctly against the scene and other spheres because we ensure the HDR framebuffer has the scene depth.
    pub fn render_sphere_at(
        &self,
        model: &glam::Mat4,
        view: &glam::Mat4,
        projection: &glam::Mat4,
        color: &glam::Vec3,
    ) {
        self.light_sphere_shader.use_program();
        self.light_sphere_shader.set_mat4("model", model);
        self.light_sphere_shader.set_mat4("view", view);
        self.light_sphere_shader.set_mat4("projection", projection);
        self.light_sphere_shader.set_vec3("color", color);
        // Compute intensity from the model's scale so intensity follows the sphere's visual radius.
        // Extract scale/rotation/translation from the model matrix and derive a scalar scale:
        let (scale, _rot, _trans) = model.to_scale_rotation_translation();
        let s = scale.x.max(scale.y).max(scale.z);
        // Choose a multiplier to produce a noticeable HDR intensity for bloom.
        // Tweak this factor if spheres appear too dim/bright.
        let intensity = s * 20.0;
        self.light_sphere_shader.set_float("intensity", intensity);

        // Ensure the HDR framebuffer has the scene depth copied into it beforehand (caller).
        // Now enable depth testing and depth writes so spheres depth-test correctly.
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthMask(gl::TRUE);
        }

        // Draw the mesh (mesh::Mesh::draw uses the provided shader only to bind VAO and draw elements)
        self.light_sphere.draw();

        // No special GL state to restore here because we left depth testing on.
    }

    pub fn blit_depth_from_gbuffer_to_hdr(&self) {
        unsafe {
            // Bind G-buffer as read framebuffer and HDR as draw, then blit depth.
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.g_buffer);
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.hdr_fbo);
            gl::BlitFramebuffer(
                0,
                0,
                self.width as i32,
                self.height as i32,
                0,
                0,
                self.width as i32,
                self.height as i32,
                gl::DEPTH_BUFFER_BIT,
                gl::NEAREST,
            );
            // Re-bind HDR framebuffer for further rendering
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.hdr_fbo);
        }
    }

    pub fn begin_directional_shadow_pass(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.directional_shadow_fbo);
            gl::Viewport(0, 0, 2048, 2048); // Shadow map resolution
            gl::Clear(gl::DEPTH_BUFFER_BIT);
            gl::CullFace(gl::FRONT); // Prevent shadow acne
        }
        self.directional_shadow_shader.use_program();
    }

    pub fn end_directional_shadow_pass(&self) {
        unsafe {
            gl::CullFace(gl::BACK); // Reset culling
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    pub fn begin_point_shadow_pass(&self, light_index: usize, face: usize) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.point_shadow_fbos[light_index][face]);
            gl::Viewport(0, 0, 1024, 1024); // Cube map face resolution
            gl::Clear(gl::DEPTH_BUFFER_BIT);
            gl::CullFace(gl::FRONT);
        }
        self.point_shadow_shader.use_program();
    }

    pub fn end_point_shadow_pass(&self) {
        unsafe {
            gl::CullFace(gl::BACK);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }
}

unsafe fn create_directional_shadow_buffer() -> (u32, u32) {
    let mut fbo = 0;
    let mut shadow_map = 0;
    unsafe {
        gl::GenFramebuffers(1, &mut fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

        gl::GenTextures(1, &mut shadow_map);
        gl::BindTexture(gl::TEXTURE_2D, shadow_map);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::DEPTH_COMPONENT as i32,
            2048,
            2048,
            0,
            gl::DEPTH_COMPONENT,
            gl::FLOAT,
            ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_S,
            gl::CLAMP_TO_BORDER as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_T,
            gl::CLAMP_TO_BORDER as i32,
        );
        let border_color = [1.0f32, 1.0f32, 1.0f32, 1.0f32];
        gl::TexParameterfv(
            gl::TEXTURE_2D,
            gl::TEXTURE_BORDER_COLOR,
            border_color.as_ptr(),
        );

        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::TEXTURE_2D,
            shadow_map,
            0,
        );
        gl::DrawBuffer(gl::NONE);
        gl::ReadBuffer(gl::NONE);

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            panic!("Directional shadow framebuffer not complete");
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }
    (fbo, shadow_map)
}

unsafe fn create_point_shadow_buffer() -> ([u32; 6], u32) {
    let mut fbos = [0u32; 6];
    let mut shadow_map = 0;
    unsafe {
        gl::GenTextures(1, &mut shadow_map);
        gl::BindTexture(gl::TEXTURE_CUBE_MAP, shadow_map);

        for i in 0..6 {
            gl::TexImage2D(
                gl::TEXTURE_CUBE_MAP_POSITIVE_X + i as u32,
                0,
                gl::DEPTH_COMPONENT as i32,
                1024,
                1024,
                0,
                gl::DEPTH_COMPONENT,
                gl::FLOAT,
                ptr::null(),
            );
        }

        gl::TexParameteri(
            gl::TEXTURE_CUBE_MAP,
            gl::TEXTURE_MIN_FILTER,
            gl::NEAREST as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_CUBE_MAP,
            gl::TEXTURE_MAG_FILTER,
            gl::NEAREST as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_CUBE_MAP,
            gl::TEXTURE_WRAP_S,
            gl::CLAMP_TO_EDGE as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_CUBE_MAP,
            gl::TEXTURE_WRAP_T,
            gl::CLAMP_TO_EDGE as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_CUBE_MAP,
            gl::TEXTURE_WRAP_R,
            gl::CLAMP_TO_EDGE as i32,
        );

        gl::GenFramebuffers(6, fbos.as_mut_ptr());
        for (i, fbo) in fbos.iter().enumerate() {
            gl::BindFramebuffer(gl::FRAMEBUFFER, *fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::TEXTURE_CUBE_MAP_POSITIVE_X + i as u32,
                shadow_map,
                0,
            );
            gl::DrawBuffer(gl::NONE);
            gl::ReadBuffer(gl::NONE);

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                panic!("Point shadow framebuffer {} not complete", i);
            }
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }
    (fbos, shadow_map)
}

unsafe fn create_g_buffer(width: u32, height: u32) -> (u32, u32, u32, u32, u32) {
    let mut g_buffer = 0;
    unsafe {
        gl::GenFramebuffers(1, &mut g_buffer);
        gl::BindFramebuffer(gl::FRAMEBUFFER, g_buffer);

        let mut g_position = 0;
        gl::GenTextures(1, &mut g_position);
        gl::BindTexture(gl::TEXTURE_2D, g_position);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA16F as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::FLOAT,
            ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            g_position,
            0,
        );

        let mut g_normal = 0;
        gl::GenTextures(1, &mut g_normal);
        gl::BindTexture(gl::TEXTURE_2D, g_normal);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA16F as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::FLOAT,
            ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT1,
            gl::TEXTURE_2D,
            g_normal,
            0,
        );

        let mut g_albedo_spec = 0;
        gl::GenTextures(1, &mut g_albedo_spec);
        gl::BindTexture(gl::TEXTURE_2D, g_albedo_spec);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT2,
            gl::TEXTURE_2D,
            g_albedo_spec,
            0,
        );

        let attachments = [
            gl::COLOR_ATTACHMENT0,
            gl::COLOR_ATTACHMENT1,
            gl::COLOR_ATTACHMENT2,
        ];
        gl::DrawBuffers(3, attachments.as_ptr());

        let mut rbo_depth = 0;
        gl::GenRenderbuffers(1, &mut rbo_depth);
        gl::BindRenderbuffer(gl::RENDERBUFFER, rbo_depth);
        gl::RenderbufferStorage(
            gl::RENDERBUFFER,
            gl::DEPTH_COMPONENT,
            width as i32,
            height as i32,
        );
        gl::FramebufferRenderbuffer(
            gl::FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::RENDERBUFFER,
            rbo_depth,
        );

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            // println!("Framebuffer not complete!");
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        (g_buffer, g_position, g_normal, g_albedo_spec, rbo_depth)
    }
}

unsafe fn create_hdr_buffer(width: u32, height: u32) -> (u32, u32) {
    // Returns (fbo, hdr_color)
    let mut fbo = 0;
    let mut color = 0;
    let mut rbo = 0;
    unsafe {
        gl::GenFramebuffers(1, &mut fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

        // Main HDR color attachment (scene lighting)
        gl::GenTextures(1, &mut color);
        gl::BindTexture(gl::TEXTURE_2D, color);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA16F as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::FLOAT,
            ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            color,
            0,
        );

        // Specify that we will draw to the color attachment (lighting shader writes to this output)
        let attachments = [gl::COLOR_ATTACHMENT0];
        gl::DrawBuffers(1, attachments.as_ptr());

        // Depth renderbuffer
        gl::GenRenderbuffers(1, &mut rbo);
        gl::BindRenderbuffer(gl::RENDERBUFFER, rbo);
        gl::RenderbufferStorage(
            gl::RENDERBUFFER,
            gl::DEPTH_COMPONENT,
            width as i32,
            height as i32,
        );
        gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::RENDERBUFFER, rbo);

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            // framebuffer incomplete
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }
    (fbo, color)
}

/// Create bloom bright-pass framebuffer & texture
unsafe fn create_bloom_buffer(width: u32, height: u32) -> (u32, u32) {
    let mut fbo = 0;
    let mut color = 0;
    unsafe {
        gl::GenFramebuffers(1, &mut fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

        gl::GenTextures(1, &mut color);
        gl::BindTexture(gl::TEXTURE_2D, color);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA16F as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::FLOAT,
            ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            color,
            0,
        );

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            panic!("Bloom framebuffer not complete");
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }
    (fbo, color)
}

/// Create two ping-pong framebuffers + textures used for separable blur
unsafe fn create_pingpong_buffers(width: u32, height: u32) -> ([u32; 2], [u32; 2]) {
    let mut fbos = [0u32; 2];
    let mut colors = [0u32; 2];
    unsafe {
        gl::GenFramebuffers(2, fbos.as_mut_ptr());
        gl::GenTextures(2, colors.as_mut_ptr());

        for (i, fbo) in fbos.iter().enumerate() {
            let color = colors[i];
            gl::BindFramebuffer(gl::FRAMEBUFFER, *fbo);
            gl::BindTexture(gl::TEXTURE_2D, color);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA16F as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::FLOAT,
                ptr::null(),
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                color,
                0,
            );

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                panic!("Pingpong framebuffer {} not complete", i);
            }
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }
    (fbos, colors)
}

unsafe fn create_quad() -> (u32, u32) {
    let quad_vertices: [f32; 20] = [
        -1.0, 1.0, 0.0, 0.0, 1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, -1.0,
        0.0, 1.0, 0.0,
    ];

    let mut quad_vao = 0;
    let mut quad_vbo = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut quad_vao);
        gl::GenBuffers(1, &mut quad_vbo);
        gl::BindVertexArray(quad_vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, quad_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (quad_vertices.len() * mem::size_of::<f32>()) as isize,
            &quad_vertices[0] as *const f32 as *const _,
            gl::STATIC_DRAW,
        );

        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            5 * mem::size_of::<f32>() as i32,
            ptr::null(),
        );

        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            5 * mem::size_of::<f32>() as i32,
            (3 * mem::size_of::<f32>()) as *const _,
        );

        gl::BindVertexArray(0);
    }
    (quad_vao, quad_vbo)
}
