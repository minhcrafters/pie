from typing import Tuple

class LightType:
    Point: int = 0
    Directional: int = 1

class Light:
    """Point or directional light."""

    position: Tuple[float, float, float]
    color: Tuple[float, float, float]
    radius: float
    light_type: LightType
    def __init__(
        self,
        r: float,
        g: float,
        b: float,
        radius: float,
        light_type: LightType,
    ) -> None: ...
    @staticmethod
    def point(r: float, g: float, b: float, radius: float) -> "Light": ...
    @staticmethod
    def directional(r: float, g: float, b: float) -> "Light": ...
