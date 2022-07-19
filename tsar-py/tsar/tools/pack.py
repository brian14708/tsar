import argparse
import pathlib
import sys

import tsar.formats.onnx
from tsar import writer


def progress(count, total, status=""):
    # https://gist.github.com/vladignatyev/06860ec2040cb497f0f3
    bar_len = 60
    filled_len = int(round(bar_len * count / float(total)))

    percents = round(100.0 * count / float(total), 1)
    bar_out = "=" * filled_len + "-" * (bar_len - filled_len)

    sys.stdout.write(f"[{bar_out}] {percents}% ...{status}\r")
    sys.stdout.flush()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("-e", "--relative-error", default=4e-3, type=float)
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
