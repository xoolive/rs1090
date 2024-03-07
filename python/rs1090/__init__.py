from __future__ import annotations

import pickle
from typing import Sequence

from ._rust import (
    decode_one,
    decode_parallel,
    decode_parallel_time,
    decode_vec,
    decode_vec_time,
)

try:
    # new in Python 3.12
    from itertools import batched  # type: ignore
except ImportError:
    from itertools import islice

    def batched(iterable, n):  # type: ignore
        # batched('ABCDEFG', 3) --> ABC DEF G
        if n < 1:
            raise ValueError("n must be at least one")
        it = iter(iterable)
        while batch := tuple(islice(it, n)):
            yield batch


__all__ = ["decode"]


def decode(
    msg: str | Sequence[str],
    timestamp: None | float | Sequence[float] = None,
    *,
    reference: None | tuple[float, float] = None,
    batch: int | None = 1000,
):
    if isinstance(msg, str):
        payload = decode_one(msg)

    else:
        if timestamp is not None and isinstance(timestamp, (int, float)):
            raise ValueError(
                "`timestamp` parameter must be a sequence of float"
            )
        if timestamp is not None and len(timestamp) != len(msg):
            raise ValueError("`msg` and `timestamp` must be of the same length")

        if batch is None:
            if timestamp is None:
                payload = decode_vec(msg)
            else:
                payload = decode_vec_time(msg, timestamp, reference)
        else:
            batches = list(batched(msg, batch))
            if timestamp is None:
                payload = decode_parallel(batches)
            else:
                ts = list(batched(timestamp, batch))
                payload = decode_parallel_time(batches, ts, reference)

    return pickle.loads(bytes(payload))
