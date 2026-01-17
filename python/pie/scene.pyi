from typing import Tuple, List, Any, TYPE_CHECKING

if TYPE_CHECKING:
    from .entity import Entity
    from .light import Light

class Scene:
    """Container for `Entity` objects and `Light` instances.

    Attributes:
        entities: List[Entity]
        lights: List[Light]
    """

    entities: List["Entity"]
    lights: List["Light"]

    def __init__(self) -> None: ...
    def add_entity(self, entity: "Entity") -> None: ...
    def add_light(self, light: "Light") -> None: ...
