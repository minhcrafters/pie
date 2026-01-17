from typing import Dict

class SdlEvent:
    """
    Python wrapper for SDL2 events produced by the engine.

    This class exposes a compact, generic representation of any SDL event.
    All event-specific fields are provided in the `fields` mapping and the
    values are represented as strings for simplicity and safety across the FFI
    boundary. Consumers can parse or cast values on the Python side as needed.

    Attributes
    ----------
    name: str
        A short textual name/variant for the SDL event (e.g. "Quit", "Window", "KeyDown").
    timestamp: int
        Event timestamp provided by SDL (0 if not available).
    fields: Dict[str, str]
        A mapping of event-specific field names to stringified values. This
        includes ALL SDL event fields so that Python code can inspect everything
        the underlying SDL event contains. Examples of keys (not exhaustive):
          - for Window events: "window_id", "event", "data1", "data2"
          - for Keyboard events: "window_id", "state", "repeat", "scancode", "keycode", "mod"
          - for Mouse events: "window_id", "which", "x", "y", "xrel", "yrel", "button", "state"
          - for TextInput: "window_id", "text"
          - for DropFile: "file"
          - for Controller events: "which", "axis", "value", "button"
          - for Audio device events: "which", "iscapture"
          - and so on for all SDL event variants
        Values are always strings; parse them in Python if you need typed values.
    repr: str
        A detailed debug-style string representation of the original event.
    """

    name: str
    timestamp: int
    fields: Dict[str, str]

    def __init__(
        self, name: str, timestamp: int = 0, fields: Dict[str, str] | None = None
    ) -> None: ...
    def __repr__(self) -> str: ...
