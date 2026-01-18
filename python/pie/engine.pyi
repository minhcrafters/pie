from typing import TYPE_CHECKING, List, Tuple

if TYPE_CHECKING:
    import pie.audio
    import pie.entity
    import pie.events
    import pie.scene
    import pie.texture

class Engine:
    """Game engine with window, renderer, input, audio, and physics."""

    camera: "pie.entity.Camera"
    def get_camera(self) -> "pie.entity.Camera":
        """Returns the active camera."""
        ...
    def set_camera(self, camera: "pie.entity.Camera") -> None:
        """Sets the active camera."""
        ...
    def __init__(self, title: str, width: int, height: int) -> None: ...
    def quit(self) -> None:
        """Quits the engine."""
    def add_entity(self, entity: "pie.entity.Entity") -> None:
        """Adds an entity to the scene."""
    def add_light(self, light: "pie.light.Light") -> None:
        """Adds a light to the scene."""
    def add_audio_source(self, source: "pie.audio.AudioSource") -> None:
        """Adds an audio source to the mixer."""
    def is_key_down(self, key: str) -> bool:
        """Returns True if the key is pressed."""
    def get_mouse_pos(self) -> Tuple[int, int]:
        """Returns mouse position."""
    def get_mouse_rel(self) -> Tuple[int, int]:
        """Returns relative mouse movement."""
    def set_mouse_capture(self, enabled: bool) -> None:
        """Enables or disables mouse capture."""
    def update(self) -> bool:
        """Updates engine state and renders. Returns False on quit."""
    def poll_events(self) -> List["pie.events.SdlEvent"]:
        """Returns and clears pending SDL events."""
        ...
    def move_camera(self, dx: float, dy: float, dz: float) -> None:
        """Moves the camera."""
    def rotate_camera(self, yaw: float, pitch: float) -> None:
        """Rotates the camera."""
    def configure_point_lights(self, count: int) -> None:
        """Configures point light shadow maps."""
