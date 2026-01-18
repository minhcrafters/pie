from typing import TYPE_CHECKING, Tuple

if TYPE_CHECKING:
    from .mesh import Mesh

class Entity:
    """A scene entity containing a `Transform` and optional mesh data.

    Attributes:
        transform: The entity's `Transform` (get/set from Python).
        mesh_attached: True when a mesh (e.g. cube) has been attached.
    """

    position: Tuple[float, float, float]
    rotation: Tuple[float, float, float]
    scale: Tuple[float, float, float]
    mesh: "Mesh"

    def __init__(self) -> None: ...
    def set_mesh(self, mesh: "Mesh") -> None: ...

class Camera:
    """Camera entity.

    Note: currently only one camera is supported, the engine's internal camera.
    """

    position: Tuple[float, float, float]
    fov: float
    yaw_pitch: Tuple[float, float]

    def __init__(self, x: float, y: float, z: float) -> None:
        """Initialize camera at position (x, y, z)."""
