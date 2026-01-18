from typing import TYPE_CHECKING, Optional, Tuple

if TYPE_CHECKING:
    import pie.texture

class Mesh:
    """3D mesh with vertex data and optional texture."""

    texture_id: int
    color: Optional[Tuple[int, int, int, int]]
    @staticmethod
    def from_obj(path: str) -> "Mesh":
        """Loads mesh from OBJ file."""
        ...
    @staticmethod
    def empty() -> "Mesh":
        """Creates empty mesh."""
        ...
    @staticmethod
    def cube() -> "Mesh":
        """Creates cube mesh."""
        ...
    @staticmethod
    def icosphere(subdivisions: int) -> "Mesh":
        """Creates icosphere mesh."""
        ...
    @staticmethod
    def plane() -> "Mesh":
        """Creates plane mesh."""
        ...
    def set_texture(self, texture: "pie.texture.Texture") -> None:
        """Attaches texture to mesh."""
        ...
    def clear_texture(self) -> None:
        """Removes attached texture."""
        ...
