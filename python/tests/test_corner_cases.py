"""
Comprehensive corner case tests for rs1090 Python bindings.

These tests mirror the Rust tests in crates/rs1090/src/decode/bds/*.rs
and validate edge cases using real ADS-B messages from flight data.
"""

from pytest import approx

import rs1090


class TestBDS06SurfacePosition:
    """Tests for BDS 0,6 (Surface Position) movement field encoding.

    Validates the fix for movement codes 13-38 which should use 0.5 kt steps,
    not 0.25 kt steps as in the buggy version.
    """

    def test_movement_no_info(self) -> None:
        """Movement code 0 should return no groundspeed information."""
        msg = rs1090.decode("8c3944f8400002acb23cda192b95")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds06(msg)
        assert "groundspeed" not in msg or msg["groundspeed"] is None

    def test_movement_stopped(self) -> None:
        """Movement code 1: aircraft stopped (0.0 kt)."""
        msg = rs1090.decode("903a33ff40100858d34ff3cce976")
        assert rs1090.is_df18(msg)
        assert rs1090.is_bds06(msg)
        assert msg["groundspeed"] == 0.0

    def test_movement_1_2kt_range(self) -> None:
        """Movement code 9: 1.0 kt (range 1.0-2.0 kt, 0.25 kt steps)."""
        msg = rs1090.decode("8c394c0f389b1667e947db7bb8bc")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds06(msg)
        assert msg["groundspeed"] == approx(1.0, abs=0.01)

    def test_movement_2_15kt_range_fix(self) -> None:
        """Movement code 25: 8.0 kt (validates 0.5 kt step fix).

        This message would decode to 5.0 kt with buggy 0.25 kt steps.
        Correct formula: 2.0 + (25-13)*0.5 = 8.0 kt
        """
        msg = rs1090.decode("8c3461cf399d6059814ea81483a9")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds06(msg)
        assert msg["groundspeed"] == approx(8.0, abs=0.01)

    def test_movement_2_15kt_range_fix_2(self) -> None:
        """Movement code 24: 7.5 kt (validates 0.5 kt step fix).

        This message would decode to 4.75 kt with buggy 0.25 kt steps.
        Correct formula: 2.0 + (24-13)*0.5 = 7.5 kt
        """
        msg = rs1090.decode("8c3461cf398d60597b4ea434c4d7")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds06(msg)
        assert msg["groundspeed"] == approx(7.5, abs=0.01)

    def test_movement_15_70kt_range(self) -> None:
        """Movement code 39: 15.0 kt (range 15-70 kt, 1.0 kt steps)."""
        msg = rs1090.decode("8c3461cf3a7f3059c94e5bf4e169")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds06(msg)
        assert msg["groundspeed"] == approx(15.0, abs=0.01)

    def test_movement_70_100kt_range(self) -> None:
        """Movement code 94: 70.0 kt (range 70-100 kt, 2.0 kt steps)."""
        msg = rs1090.decode("8c3950cf3dede47bac304d3b5122")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds06(msg)
        assert msg["groundspeed"] == approx(70.0, abs=0.01)

    def test_movement_100_175kt_range(self) -> None:
        """Movement code 109: 100.0 kt (range 100-175 kt, 5.0 kt steps)."""
        msg = rs1090.decode("8c3933203edde47b9e2ffa5e77b8")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds06(msg)
        assert msg["groundspeed"] == approx(100.0, abs=0.01)

    def test_movement_175kt_plus(self) -> None:
        """Movement code 124: 175.0 kt (≥175 kt)."""
        msg = rs1090.decode("8d3933203fcde2a84e39e1c6c5bc")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds06(msg)
        assert msg["groundspeed"] == approx(175.0, abs=0.01)

    def test_track_invalid(self) -> None:
        """Track status = 0 should return no track information."""
        msg = rs1090.decode("903a33ff40100858d34ff3cce976")
        assert rs1090.is_df18(msg)
        assert rs1090.is_bds06(msg)
        # Track should be None or not present when status is invalid
        assert "track" not in msg or msg["track"] is None


class TestBDS09AirborneVelocity:
    """Tests for BDS 0,9 (Airborne Velocity) vertical rate encoding.

    Validates sign bit handling and edge cases for vertical rate.
    """

    def test_vertical_rate_positive_64(self) -> None:
        """Vertical rate +64 ft/min (minimum positive rate)."""
        msg = rs1090.decode("8d3461cf9908388930080f948ea1")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds09(msg)
        assert msg["vertical_rate"] == 64
        assert msg["vrate_src"] == "GNSS"

    def test_vertical_rate_positive_128(self) -> None:
        """Vertical rate +128 ft/min."""
        msg = rs1090.decode("8d3461cf9908558e100c1071eb67")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds09(msg)
        assert msg["vertical_rate"] == 128
        assert msg["vrate_src"] == "GNSS"

    def test_vertical_rate_positive_960(self) -> None:
        """Vertical rate +960 ft/min."""
        msg = rs1090.decode("8d3461cf99085a8f10400f80e6ac")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds09(msg)
        assert msg["vertical_rate"] == 960
        assert msg["vrate_src"] == "GNSS"

    def test_vertical_rate_negative_64(self) -> None:
        """Vertical rate -64 ft/min (minimum negative rate)."""
        msg = rs1090.decode("8d394c0f990c4932780838866883")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds09(msg)
        assert msg["vertical_rate"] == -64
        assert msg["vrate_src"] == "GNSS"

    def test_vertical_rate_sign_bit(self) -> None:
        """Verify sign bit handling: same magnitude, opposite signs."""
        msg_pos = rs1090.decode("8d3461cf9908388930080f948ea1")  # +64
        msg_neg = rs1090.decode("8d394c0f990c4932780838866883")  # -64

        assert rs1090.is_df17(msg_pos)
        assert rs1090.is_df17(msg_neg)
        assert rs1090.is_bds09(msg_pos)
        assert rs1090.is_bds09(msg_neg)

        assert msg_pos["vertical_rate"] == 64
        assert msg_neg["vertical_rate"] == -64
        assert msg_pos["vertical_rate"] == -msg_neg["vertical_rate"]

    def test_vrate_source_gnss(self) -> None:
        """Vertical rate from GNSS source."""
        msg = rs1090.decode("8d3461cf9908388930080f948ea1")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds09(msg)
        assert msg["vrate_src"] == "GNSS"

    def test_vrate_source_barometric(self) -> None:
        """Vertical rate from barometric source."""
        msg = rs1090.decode("8D485020994409940838175B284F")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds09(msg)
        assert msg["vrate_src"] == "barometric"

    def test_geo_minus_baro_positive(self) -> None:
        """GNSS altitude above barometric (+550 ft)."""
        msg = rs1090.decode("8D485020994409940838175B284F")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds09(msg)
        assert msg["geo_minus_baro"] == 550


class TestBDS05AirbornePosition:
    """Tests for BDS 0,5 (Airborne Position) altitude encoding.

    Validates altitude decoding including negative values for airports
    below sea level (e.g., Amsterdam EHAM at -11 ft MSL).
    """

    def test_negative_altitude_325ft(self) -> None:
        """Altitude -325 ft (below sea level)."""
        msg = rs1090.decode("8d484fde5803b647ecec4fcdd74f")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds05(msg)
        assert msg["altitude"] == -325

    def test_negative_altitude_300ft(self) -> None:
        """Altitude -300 ft (below sea level)."""
        msg = rs1090.decode("8d4845575803c647bcec2a980abc")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds05(msg)
        assert msg["altitude"] == -300

    def test_negative_altitude_275ft(self) -> None:
        """Altitude -275 ft (below sea level)."""
        msg = rs1090.decode("8d3424d25803d64c18ee03351f89")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds05(msg)
        assert msg["altitude"] == -275

    def test_zero_altitude(self) -> None:
        """Altitude 0 ft (sea level)."""
        msg = rs1090.decode("8d4401e458058645a8ea90496290")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds05(msg)
        assert msg["altitude"] == 0

    def test_small_positive_altitude_25ft(self) -> None:
        """Altitude 25 ft (low altitude)."""
        msg = rs1090.decode("8d346355580596459cea86756acc")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds05(msg)
        assert msg["altitude"] == 25

    def test_positive_altitude_1000ft(self) -> None:
        """Altitude 1000 ft."""
        msg = rs1090.decode("8d346355580b064116e70a269f97")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds05(msg)
        assert msg["altitude"] == 1000

    def test_positive_altitude_5000ft(self) -> None:
        """Altitude 5000 ft."""
        msg = rs1090.decode("8d343386581f06318ad4fecab734")
        assert rs1090.is_df17(msg)
        assert rs1090.is_bds05(msg)
        assert msg["altitude"] == 5000

    # Note: test_altitude_all_zeros is skipped because we don't have a real
    # message with altitude field = 0x000 in our sample data.
    # The Rust tests use a synthetic message for this edge case.
