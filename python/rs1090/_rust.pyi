from typing import Sequence

def aircraft_information(
    icao24: str, registration: None | str = None
) -> dict[str, str]: ...
def decode_1090(msg: str) -> list[int]: ...
def decode_1090_vec(msgs: Sequence[Sequence[str]]) -> list[int]: ...
def decode_1090t_vec(
    msgs: Sequence[Sequence[str]],
    ts: Sequence[Sequence[float]],
    reference: None | tuple[float, float] = None,
) -> list[int]: ...
def decode_flarm(
    msg: str, timestamp: int, reflat: float, reflon: float
) -> list[int]: ...
def decode_flarm_vec(
    msgs: Sequence[Sequence[str]],
    ts: Sequence[Sequence[int]],
    reflat: Sequence[Sequence[float]],
    reflon: Sequence[Sequence[float]],
) -> list[int]: ...
