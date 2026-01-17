from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import pie.texture

class Mesh:
    @staticmethod
    def from_obj(path: str) -> "Mesh":
        """Loads a mesh from an OBJ file located at `path`."""

    @staticmethod
    def empty() -> "Mesh":
        """Creates an empty mesh (no vertices)."""

    @staticmethod
    def cube() -> "Mesh":
        """Creates a cube mesh."""

    @staticmethod
    def icosphere(subdivisions: int) -> "Mesh":
        """Creates an icosphere mesh with the given number of subdivisions."""

    @staticmethod
    def plane() -> "Mesh":
        """Creates a plane mesh."""

    def set_texture(self, texture: "pie.texture.Texture") -> None:
        """Attach a color Texture to this mesh.

        The provided `Texture` is treated as a solid RGBA color (r,g,b,a channels 0-255).
        The mesh will use the texture's color as its albedo (rgb) and the alpha channel
        as a specular intensity hint. Image-based GPU sampling has been removed; this
        preserves the previous API shape while switching to color-only mapping.
        """

    def clear_texture(self) -> None:
        """Remove any attached color from this mesh (revert to default material color)."""
