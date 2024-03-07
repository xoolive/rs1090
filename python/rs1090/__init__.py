import itertools
import pickle
from typing import Sequence

from ._rust import (
    decode_one,
    decode_parallel,
    decode_parallel_time,
    decode_vec,
    decode_vec_time,
)

# import msgpack
#     payload = decode_msgpack(msg)
#     return msgpack.unpackb(bytes(payload))


def decode(
    msg: str | Sequence[str],
    timestamp: None | float | Sequence[float] = None,
    *,
    batch: int | None = 1000,
):
    if isinstance(msg, str):
        payload = decode_one(msg)

    else:
        if timestamp is not None and isinstance(timestamp, (int, float)):
            raise ValueError("`timestamp` parameter must be a sequence of float")
        if timestamp is not None and len(timestamp) != len(msg):
            raise ValueError("`msg` and `timestamp` must be of the same length")

        if batch is None:
            if timestamp is None:
                payload = decode_vec(msg)
            else:
                payload = decode_vec_time(msg, timestamp)
        else:
            batches = list(itertools.batched(msg, batch))
            if timestamp is None:
                payload = decode_parallel(batches)
            else:
                ts = list(itertools.batched(timestamp, batch))
                payload = decode_parallel_time(batches, ts)

    return pickle.loads(bytes(payload))
