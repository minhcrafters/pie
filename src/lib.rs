use pyo3::prelude::*;

pub mod audio;
pub mod engine;
pub mod input;
pub mod physics;
pub mod renderer;
pub mod scene;

#[pymodule]
fn pie(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sys_modules = py.import("sys")?.getattr("modules")?;

    let engine_mod = PyModule::new(py, "pie.engine")?;
    engine_mod.add_class::<engine::Engine>()?;
    m.add_submodule(&engine_mod)?;
    sys_modules.set_item("pie.engine", &engine_mod)?;

    let mesh_mod = PyModule::new(py, "pie.mesh")?;
    mesh_mod.add_class::<renderer::mesh::Mesh>()?;
    m.add_submodule(&mesh_mod)?;
    sys_modules.set_item("pie.mesh", &mesh_mod)?;

    let texture_mod = PyModule::new(py, "pie.texture")?;
    texture_mod.add_class::<renderer::texture::Texture>()?;
    m.add_submodule(&texture_mod)?;
    sys_modules.set_item("pie.texture", &texture_mod)?;

    let scene_mod = PyModule::new(py, "pie.scene")?;
    scene_mod.add_class::<scene::Scene>()?;
    m.add_submodule(&scene_mod)?;
    sys_modules.set_item("pie.scene", &scene_mod)?;

    let entity_mod = PyModule::new(py, "pie.entity")?;
    entity_mod.add_class::<scene::Entity>()?;
    entity_mod.add_class::<scene::Camera>()?;
    m.add_submodule(&entity_mod)?;
    sys_modules.set_item("pie.entity", &entity_mod)?;

    let light_mod = PyModule::new(py, "pie.light")?;
    light_mod.add_class::<scene::Light>()?;
    light_mod.add_class::<scene::LightType>()?;
    m.add_submodule(&light_mod)?;
    sys_modules.set_item("pie.light", &light_mod)?;

    let audio_mod = PyModule::new(py, "pie.audio")?;
    audio_mod.add_class::<audio::AudioSource>()?;
    m.add_submodule(&audio_mod)?;
    sys_modules.set_item("pie.audio", &audio_mod)?;

    Ok(())
}
