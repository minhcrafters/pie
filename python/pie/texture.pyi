from typing import Tuple

class Texture:
    """
    Color-only Texture object.

    Represents a solid RGBA color (0-255 channels). Image-based GPU textures and sampling
    have been removed; this keeps a compatibility shim so existing code using
    `Texture.from_color(...)` still works.
    """

    # Public fields exposed from the Rust side (color channels 0-255)
    r: int
    g: int
    b: int
    a: int

    @staticmethod
    def from_color(r: int, g: int, b: int, a: int) -> "Texture":
        """
        Create a color "texture" from the specified RGBA color channels (0-255).
        """

    @staticmethod
    def from_image(path: str) -> "Texture":
        """
        Legacy shim: image loading is not supported in color-only mode. Returns a magenta
        debug color Texture to indicate a missing/unsupported image.
        """

    def bind(self, unit: int) -> None:
        """
        No-op compatibility method; color mapping doesn't bind GL textures.
        """

    def unbind(self) -> None:
        """
        No-op compatibility method.
        """

    def to_rgba_f32(self) -> Tuple[float, float, float, float]:
        """Return color channels normalized to 0.0-1.0 floats."""

    def get_width(self) -> int:
        """Return texture width in pixels (1 for color-only Texture)."""

    def get_height(self) -> int:
        """Return texture height in pixels (1 for color-only Texture)."""
