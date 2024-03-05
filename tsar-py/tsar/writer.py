from contextlib import contextmanager

from .tsar import Writer


@contextmanager
def writer(*args, **kwds):
    # Code to acquire resource, e.g.:
    wobj = Writer(*args, **kwds)
    try:
        yield wobj
    finally:
        wobj.close()
