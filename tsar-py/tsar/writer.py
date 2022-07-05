from contextlib import contextmanager

import tsar.tsar as _tsar


@contextmanager
def writer(*args, **kwds):
    # Code to acquire resource, e.g.:
    wobj = _tsar.Writer(*args, **kwds)
    try:
        yield wobj
    finally:
        wobj.close()
