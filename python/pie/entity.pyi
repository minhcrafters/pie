from typing import TYPE_CHECKING, Tuple

if TYPE_CHECKING:
    from .mesh import Mesh

class Entity:
    """Scene entity with transform and optional mesh."""

    position: Tuple[float, float, float]
    rotation: Tuple[float, float, float]
    scale: Tuple[float, float, float]
    mesh: "Mesh"
    def __init__(self) -> None: ...
    def set_mesh(self, mesh: "Mesh") -> None: ...

class Camera:
    """Camera entity."""

    position: Tuple[float, float, float]
    fov: float
    yaw_pitch: Tuple[float, float]
    def __init__(self, x: float, y: float, z: float) -> None:
        """Creates camera at position."""
