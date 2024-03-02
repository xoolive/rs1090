import msgpack

from ._rust import decode as rust_decode


def decode(msg: str):
    payload = rust_decode(msg)
    return msgpack.unpackb(bytes(payload))
