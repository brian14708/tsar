from typing import Optional

class Writer:
    def __init__(self, dst: str): ...
    def write_blob(
        self,
        type: str,
        name: str,
        offset: int,
        data: bytes,
        shape: list[int],
        level: int,
        error: float,
    ) -> None: ...
    def write_file(
        self,
        name: str,
        data: bytes,
    ) -> None: ...
