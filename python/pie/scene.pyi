from typing import TYPE_CHECKING, List

if TYPE_CHECKING:
    from .entity import Entity
    from .light import Light

class Scene:
    """Container for entities and lights."""

    entities: List["Entity"]
    lights: List["Light"]
    def __init__(self) -> None: ...
    def add_entity(self, entity: "Entity") -> None: ...
    def add_light(self, light: "Light") -> None: ...
