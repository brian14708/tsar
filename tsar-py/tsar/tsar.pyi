from typing import Optional, Tuple

class Writer:
    def __init__(self, dst: str): ...
    def write_blob(
        self,
        type: str,
        name: str,
        data: bytes,
        shape: list[int],
        error: float,
        target_file: Optional[Tuple[str, int]],
    ) -> None: ...
    def write_file(
        self,
        name: str,
        data: bytes,
    ) -> None: ...

class Reader:
    def __init__(self, src: str): ...
    def extract_files(self, dst: str): ...
    def extract_blobs(self, dst: str): ...
