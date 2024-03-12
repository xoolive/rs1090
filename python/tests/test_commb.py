from pytest import approx

import rs1090


def test_bds20() -> None:
    msg = rs1090.decode("A000083E202CC371C31DE0AA1CCF")
    assert rs1090.is_df20(msg)
    assert rs1090.is_bds20(msg)
    assert msg["callsign"] == "KLM1017"
    msg = rs1090.decode("A0001993202422F2E37CE038738E")
    assert rs1090.is_df20(msg)
    assert rs1090.is_bds20(msg)
    assert msg["callsign"] == "IBK2873"


def test_bds40() -> None:
    msg = rs1090.decode("A000029C85E42F313000007047D3")
    assert rs1090.is_df20(msg)
    assert rs1090.is_bds40(msg)
    assert msg["selected_mcp"] == 3000
    assert msg["selected_fms"] == 3000
    assert msg["barometric_setting"] == 1020


def test_bds50() -> None:
    msg = rs1090.decode("A000139381951536E024D4CCF6B5")
    assert rs1090.is_df20(msg)
    assert rs1090.is_bds50(msg)
    assert msg["roll"] == approx(2.11, rel=1e-3)
    assert msg["track"] == approx(114.2578)
    assert msg["groundspeed"] == 438
    assert msg["track_rate"] == 0.125
    assert msg["TAS"] == 424


def test_bds60() -> None:
    msg = rs1090.decode("A00004128F39F91A7E27C46ADC21")
    assert rs1090.is_df20(msg)
    assert rs1090.is_bds60(msg)
    assert msg["heading"] == approx(42.71484)
    assert msg["IAS"] == 252
    assert msg["Mach"] == 0.42
    assert msg["vrate_barometric"] == -1920
    assert msg["vrate_inertial"] == -1920
