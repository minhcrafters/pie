from typing import List, Tuple

class AudioSource:
    """Audio source."""

    position: Tuple[float, float, float]
    positional: bool
    looping: bool
    playing: bool
    cursor: int
    duration: float

    @staticmethod
    def new_sine(freq: float, looping: bool) -> "AudioSource": ...
    @staticmethod
    def new_clip(samples: List[float], looping: bool) -> "AudioSource": ...
    @staticmethod
    def from_wav(file: str, looping: bool) -> "AudioSource": ...
    def play(self) -> None: ...
    def pause(self) -> None: ...
    def is_playing(self) -> bool: ...
