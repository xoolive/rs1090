import rs1090


def test_crc() -> None:
    assert rs1090.decode("8D406B902015A678D4D220AA4BDA")["df"] == "17"
    assert rs1090.decode("8d8960ed58bf053cf11bc5932b7d")["df"] == "17"
    assert rs1090.decode("8d45cab390c39509496ca9a32912")["df"] == "17"
    assert rs1090.decode("8d49d3d4e1089d00000000744c3b")["df"] == "17"
    assert rs1090.decode("8d74802958c904e6ef4ba0184d5c")["df"] == "17"
    assert rs1090.decode("8d4400cd9b0000b4f87000e71a10")["df"] == "17"
    assert rs1090.decode("8d4065de58a1054a7ef0218e226a")["df"] == "17"


def test_fail_crc() -> None:
    assert rs1090.decode("8d4ca251204994b1c36e60a5343d") is None


def test_icao24() -> None:
    assert rs1090.decode("8D406B902015A678D4D220AA4BDA")["icao24"] == "406b90"
    assert rs1090.decode("A0001839CA3800315800007448D9")["icao24"] == "400940"
    assert rs1090.decode("A000139381951536E024D4CCF6B5")["icao24"] == "3c4dd2"
    assert rs1090.decode("A000029CFFBAA11E2004727281F1")["icao24"] == "4243d0"


def test_altcode() -> None:
    msg = rs1090.decode("A02014B400000000000000F9D514")
    assert rs1090.is_df20(msg)
    assert msg["altitude"] == 32300


def test_idcode() -> None:
    msg = rs1090.decode("A800292DFFBBA9383FFCEB903D01")
    assert rs1090.is_df21(msg)
    assert msg["squawk"] == "1346"
