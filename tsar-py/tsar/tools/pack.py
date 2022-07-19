import argparse
import pathlib
import sys

import tsar.formats.onnx
from tsar import writer


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("-e", "--relative-error", default=1e-3, type=float)
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
                    args.relative_error,
                )
            else:
                print("unable to autodetect format: %s (supported: onnx)", src.suffix)
                sys.exit(1)
