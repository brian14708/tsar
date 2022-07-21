import sys


def progress(count, total, status=""):
    # https://gist.github.com/vladignatyev/06860ec2040cb497f0f3
    bar_len = 60
    filled_len = int(round(bar_len * count / float(total)))

    percents = round(100.0 * count / float(total), 1)
    bar_out = "=" * filled_len + "-" * (bar_len - filled_len)

    sys.stdout.write(f"[{bar_out}] {percents}% ...{status}\r")
    sys.stdout.flush()
