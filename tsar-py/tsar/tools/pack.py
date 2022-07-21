import argparse
import pathlib
import sys

import tsar.formats.onnx
from tsar import writer
from tsar.utils.progress import progress


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("-e", "--error", default=1e-6, type=float)
    parser.add_argument("srcs", nargs="+", metavar="INPUT", type=pathlib.Path)
    parser.add_argument("dst", metavar="OUTPUT", type=pathlib.Path)
    args = parser.parse_args()

    with writer(str(args.dst)) as wobj:
        for src in args.srcs:
            print(f"Processing {src}...")
            if src.suffix == ".onnx":
                tsar.formats.onnx.save(
                    src.name,
                    src,
                    wobj,
                    args.error,
                    progress_fn=progress,
                )
            elif src.suffix == ".json":
                with open(src, "rb") as fobj:
                    wobj.write_file(src.name, fobj.read())
            else:
                print(
                    "unable to autodetect format: %s (supported: onnx, json)",
                    src.suffix,
                )
                sys.exit(1)
