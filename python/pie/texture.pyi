from typing import Tuple

class Texture:
    """
    Texture object (color-only for now).

    Represents a solid RGBA color (0-255 channels).
    """

    r: int
    g: int
    b: int
    a: int

    @staticmethod
    def from_color(r: int, g: int, b: int, a: int) -> "Texture":
        """
        Create a color "texture" from the specified RGBA color channels (0-255).
        """

    def to_rgba_f32(self) -> Tuple[float, float, float, float]:
        """Return color channels normalized to 0.0-1.0 floats."""

    def get_width(self) -> int:
        """Return texture width in pixels (1 for color-only Texture)."""

    def get_height(self) -> int:
        """Return texture height in pixels (1 for color-only Texture)."""
