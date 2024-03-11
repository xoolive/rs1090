from __future__ import annotations

import pickle
from typing import Sequence

from ._rust import (
    decode_1090,
    decode_1090_vec,
    decode_1090t_vec,
    decode_flarm,
    decode_flarm_vec,
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


__all__ = ["decode", "flarm"]


def decode(
    msg: str | Sequence[str],
    timestamp: None | float | Sequence[float] = None,
    *,
    reference: None | tuple[float, float] = None,
    batch: int = 1000,
):
    if isinstance(msg, str):
        payload = decode_1090(msg)

    else:
        if timestamp is not None and isinstance(timestamp, (int, float)):
            raise ValueError(
                "`timestamp` parameter must be a sequence of float"
            )
        if timestamp is not None and len(timestamp) != len(msg):
            raise ValueError("`msg` and `timestamp` must be of the same length")

        batches = list(batched(msg, batch))
        if timestamp is None:
            payload = decode_1090_vec(batches)
        else:
            ts = list(batched(timestamp, batch))
            payload = decode_1090t_vec(batches, ts, reference)

    return pickle.loads(bytes(payload))


def flarm(
    msg: str | Sequence[str],
    timestamp: int | Sequence[int],
    reference_latitude: float | Sequence[float],
    reference_longitude: float | Sequence[float],
    *,
    batch: int = 1000,
):
    if isinstance(msg, str):
        assert isinstance(timestamp, (int, float))
        assert isinstance(reference_latitude, (int, float))
        assert isinstance(reference_longitude, (int, float))
        payload = decode_flarm(
            msg,
            timestamp,
            reference_latitude,
            reference_longitude,
        )
    else:
        batches = list(batched(msg, batch))
        assert not isinstance(timestamp, (int, float))
        assert not isinstance(reference_latitude, (int, float))
        assert not isinstance(reference_longitude, (int, float))

        t = list(batched(timestamp, batch))
        reflat = list(batched(reference_latitude, batch))
        reflon = list(batched(reference_longitude, batch))

        payload = decode_flarm_vec(batches, t, reflat, reflon)

    return pickle.loads(bytes(payload))
