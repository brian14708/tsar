import argparse
import pathlib

from tsar import extract


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("src", metavar="INPUT", type=pathlib.Path)
    parser.add_argument("dst", metavar="OUTPUT", type=pathlib.Path)
    args = parser.parse_args()
    extract(args.src, args.dst)
