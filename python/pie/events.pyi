from typing import Dict

class SdlEvent:
    """SDL2 event wrapper."""

    name: str
    timestamp: int
    fields: Dict[str, str]
    def __init__(
        self, name: str, timestamp: int = 0, fields: Dict[str, str] | None = None
    ) -> None: ...
    def __repr__(self) -> str: ...
