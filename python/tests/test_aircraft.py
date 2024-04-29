from rs1090 import aircraft_information


def test_info() -> None:
    info = aircraft_information("39B415")
    assert info == {
        "country": "France",
        "registration": "F-HNAV",
        "pattern": "^F-",
        "icao24": "39b415",
        "flag": "🇫🇷",
    }
