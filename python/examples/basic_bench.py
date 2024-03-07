# ruff: noqa: E402
# %%
from rs1090 import decode

msg = "8DA05F219B06B6AF189400CBC33F"
x = decode(msg)
x

# %%
import timeit

number = 10_000
t = timeit.timeit("x = decode(msg)", number=number, globals=globals()) / number
t


# %%
import pyModeS as pms

pms.tell(msg)

# %%
import contextlib
import sys


class DummyFile(object):
    def write(self, x):
        pass


@contextlib.contextmanager
def nostdout():
    save_stdout = sys.stdout
    sys.stdout = DummyFile()
    yield
    sys.stdout = save_stdout


# %%


def pymodes():
    with nostdout():
        pms.tell(msg)


t = timeit.timeit(pymodes, number=number, globals=globals()) / number
t
