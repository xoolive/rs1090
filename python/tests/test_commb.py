from pytest import approx

import rs1090


def test_bds20() -> None:
    msg = rs1090.decode("A000083E202CC371C31DE0AA1CCF")
    assert rs1090.is_df20(msg)
    bds20 = msg["bds20"]
    assert bds20 is not None
    assert bds20["callsign"] == "KLM1017"
    msg = rs1090.decode("A0001993202422F2E37CE038738E")
    assert rs1090.is_df20(msg)
    bds20 = msg["bds20"]
    assert bds20 is not None
    assert bds20["callsign"] == "IBK2873"


def test_bds40() -> None:
    msg = rs1090.decode("A000029C85E42F313000007047D3")
    assert rs1090.is_df20(msg)
    bds40 = msg["bds40"]
    assert bds40 is not None
    assert bds40["selected_mcp"] == 3000
    assert bds40["selected_fms"] == 3000
    assert bds40["barometric_setting"] == 1020


def test_bds50() -> None:
    msg = rs1090.decode("A000139381951536E024D4CCF6B5")
    assert rs1090.is_df20(msg)
    bds50 = msg["bds50"]
    assert bds50 is not None
    assert bds50["roll"] == approx(2.11, rel=1e-3)
    assert bds50["track"] == approx(114.2578)
    assert bds50["groundspeed"] == 438
    assert bds50["track_rate"] == 0.125
    assert bds50["TAS"] == 424


def test_bds60() -> None:
    msg = rs1090.decode("A00004128F39F91A7E27C46ADC21")
    assert rs1090.is_df20(msg)
    bds60 = msg["bds60"]
    assert bds60 is not None
    assert bds60["heading"] == approx(42.71484)
    assert bds60["IAS"] == 252
    assert bds60["Mach"] == 0.42
    assert bds60["vrate_barometric"] == -1920
    assert bds60["vrate_inertial"] == -1920
