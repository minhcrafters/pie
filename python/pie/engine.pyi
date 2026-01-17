from typing import TYPE_CHECKING, List, Tuple

if TYPE_CHECKING:
    import pie.audio
    import pie.entity
    import pie.events
    import pie.scene
    import pie.texture

class Engine:
    """Main engine object: window, renderer, input, audio, physics.

    Typical usage:
        engine = Engine("Title", 800, 600)
        engine.add_entity(entity)
        running = True
        while running:
            running = engine.update()

    Note: GPU textures and helpers are exposed via the `pie.texture` submodule
    (for example `pie.texture.Texture.from_image(...)`).
    """

    camera: "pie.entity.Camera"

    def get_camera(self) -> "pie.entity.Camera":
        """Return the current active camera instance."""
        ...

    def set_camera(self, camera: "pie.entity.Camera") -> None:
        """Set the active camera for the engine."""
        ...

    def __init__(self, title: str, width: int, height: int) -> None: ...
    def quit(self) -> None:
        """Exits."""

    def add_entity(self, entity: "pie.entity.Entity") -> None:
        """Add an `Entity` instance to the internal scene."""

    def add_light(self, light: "pie.light.Light") -> None:
        """Add a `Light` to the scene."""

    def add_audio_source(self, source: "pie.audio.AudioSource") -> None:
        """Add a pre-created `AudioSource` to the engine's mixer."""

    def is_key_down(self, key: str) -> bool:
        """Return True if `key` is currently pressed. Use names like 'W', 'S', 'Escape', 'Return'."""

    def get_mouse_pos(self) -> Tuple[int, int]:
        """Return the absolute mouse position (x, y)."""

    def get_mouse_rel(self) -> Tuple[int, int]:
        """Return relative mouse movement (dx, dy) since last frame."""

    def set_mouse_capture(self, enabled: bool) -> None:
        """Enable or disable relative mouse capture."""

    def update(self) -> bool:
        """Poll events, step physics, render a frame. Returns False to signal quit."""

    def poll_events(self) -> List["pie.events.SdlEvent"]:
        """Return and clear pending SDL events (list of SdlEvent)."""
        ...

    def move_camera(self, dx: float, dy: float, dz: float) -> None:
        """Translate camera by (dx, dy, dz)."""

    def rotate_camera(self, yaw: float, pitch: float) -> None:
        """Rotate camera by yaw and pitch (degrees)."""

    def configure_point_lights(self, count: int) -> None:
        """Configure the number of point lights with shadow maps.

        Must be called before adding any point lights. `count` is the number
        of point lights that will cast shadows (max 8).
        """
