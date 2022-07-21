import pathlib

from tsar.tsar import Reader


def extract(src: pathlib.Path, dst: pathlib.Path):
    rdr = Reader(str(src))
    rdr.extract_files(str(dst))
    rdr.extract_blobs(str(dst))
