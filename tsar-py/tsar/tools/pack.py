import argparse
import pathlib
import logging
import sys

import tsar.formats.onnx


def main() -> None:
    parser = argparse.ArgumentParser(description="Process some integers.")
    parser.add_argument(
        "--format", default="autodetect", type=str, choices=["autodetect", "onnx"]
    )
    parser.add_argument("src", metavar="INPUT", type=pathlib.Path)
    parser.add_argument("dst", metavar="OUTPUT", type=pathlib.Path)
    args = parser.parse_args()

    if args.format == "autodetect":
        if args.src.suffix == ".onnx":
            args.format = "onnx"
        else:
            logging.error("unable to autodetect format: %s", args.src.suffix)
            sys.exit(1)

    if args.format == "onnx":
        tsar.formats.onnx.save(args.src.name, args.src, args.dst)
