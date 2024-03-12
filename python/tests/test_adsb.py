from pytest import approx

import rs1090


def test_icao24() -> None:
    assert rs1090.decode("8D406B902015A678D4D220AA4BDA")["icao24"] == "406b90"


def test_wake_vortex() -> None:
    msg = rs1090.decode("8D406B902015A678D4D220AA4BDA")
    assert rs1090.is_df17(msg)
    assert rs1090.is_bds08(msg)
    assert msg["wake_vortex"] == "n/a"


def test_adsb_callsign() -> None:
    msg = rs1090.decode("8D406B902015A678D4D220AA4BDA")
    assert rs1090.is_df17(msg)
    assert rs1090.is_bds08(msg)
    assert msg["callsign"] == "EZY85MH"


def test_adsb_alt() -> None:
    msg = rs1090.decode("8D40058B58C901375147EFD09357")
    assert rs1090.is_df17(msg)
    assert rs1090.is_bds05(msg)
    assert msg["altitude"] == 39000


def test_adsb_velocity() -> None:
    msg = rs1090.decode("8D485020994409940838175B284F")
    assert rs1090.is_df17(msg)
    assert rs1090.is_bds09(msg)
    assert msg["groundspeed"] == approx(159.2, rel=1e-3)
    assert msg["vertical_rate"] == -832
    assert msg["track"] == approx(182.88, rel=1e-3)
    assert msg["geo_minus_baro"] == 550

    msg = rs1090.decode("8DA05F219B06B6AF189400CBC33F")
    assert rs1090.is_df17(msg)
    assert rs1090.is_bds09(msg)
    assert msg["TAS"] == 375
    assert msg["vertical_rate"] == -2304
    assert msg["heading"] == approx(243.98, rel=1e-3)


def test_adsb_emergency() -> None:
    msg = rs1090.decode("8DA2C1B6E112B600000000760759")
    assert rs1090.is_df17(msg)
    assert rs1090.is_bds61(msg)
    assert msg["emergency_state"] == "none"
    assert msg["squawk"] == "6513"


def test_adsb_target_state_status() -> None:
    msg = rs1090.decode("8DA05629EA21485CBF3F8CADAEEB")
    assert rs1090.is_df17(msg)
    assert rs1090.is_bds62(msg)
    assert msg["selected_altitude"] == 17000
    assert msg["source"] == "MCP/FCU"
    assert msg["barometric_setting"] == approx(1012.8)
    assert msg["selected_heading"] == approx(66.8, 0.1)
    assert msg["autopilot"]
    assert msg["vnav_mode"]
    assert not msg["alt_hold"]
    assert not msg["approach_mode"]
    assert msg["lnav_mode"]
    assert msg["tcas_operational"]
