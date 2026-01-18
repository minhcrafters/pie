from typing import Tuple

class Texture:
    """Texture object for images or solid colors."""

    id: int
    r: int
    g: int
    b: int
    a: int
    @staticmethod
    def from_color(r: int, g: int, b: int, a: int) -> "Texture":
        """Creates texture from RGBA color."""
        ...
    @staticmethod
    def from_image(path: str) -> "Texture":
        """Loads texture from image file."""
        ...
    def to_rgba_f32(self) -> Tuple[float, float, float, float]:
        """Returns color as normalized floats."""
        ...
