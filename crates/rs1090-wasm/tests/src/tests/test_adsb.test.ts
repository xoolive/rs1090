import { decode } from "../index";
import { expect, describe, test } from "@jest/globals";

describe("ADSB Decoding Tests", () => {
  test("icao24", () => {
    const msg = decode("8D406B902015A678D4D220AA4BDA");
    expect(msg.icao24).toBe("406b90");
  });

  test("wake vortex", () => {
    const msg = decode("8D406B902015A678D4D220AA4BDA");
    expect(msg.df).toBe("17");
    expect(msg.bds).toBe("08");
    expect(msg.wake_vortex).toBe("n/a");
  });

  test("adsb callsign", () => {
    const msg = decode("8D406B902015A678D4D220AA4BDA");
    expect(msg.df).toBe("17");
    expect(msg.bds).toBe("08");
    expect(msg.callsign).toBe("EZY85MH");
  });

  test("adsb altitude", () => {
    const msg = decode("8D40058B58C901375147EFD09357");
    expect(msg.df).toBe("17");
    expect(msg.bds).toBe("05");
    expect(msg.altitude).toBe(39000);
  });

  test("adsb velocity", () => {
    const msg = decode("8D485020994409940838175B284F");
    expect(msg.df).toBe("17");
    expect(msg.bds).toBe("09");
    expect(msg.groundspeed).toBeCloseTo(159.2, 1);
    expect(msg.vertical_rate).toBe(-832);
    expect(msg.track).toBeCloseTo(182.88, 3);
    expect(msg.geo_minus_baro).toBe(550);

    const msg2 = decode("8DA05F219B06B6AF189400CBC33F");
    expect(msg2.df).toBe("17");
    expect(msg2.bds).toBe("09");
    expect(msg2.TAS).toBe(375);
    expect(msg2.vertical_rate).toBe(-2304);
    expect(msg2.heading).toBeCloseTo(243.98, 1);
  });

  test("adsb emergency", () => {
    const msg = decode("8DA2C1B6E112B600000000760759");
    expect(msg.df).toBe("17");
    expect(msg.bds).toBe("61");
    expect(msg.emergency_state).toBe("none");
    expect(msg.squawk).toBe("6513");
  });

  test("adsb target state status", () => {
    const msg = decode("8DA05629EA21485CBF3F8CADAEEB");
    expect(msg.df).toBe("17");
    expect(msg.bds).toBe("62");
    expect(msg.selected_altitude).toBe(17000);
    expect(msg.source).toBe("MCP/FCU");
    expect(msg.barometric_setting).toBeCloseTo(1012.8);
    expect(msg.selected_heading).toBeCloseTo(66.8, 0.1);
    expect(msg.autopilot).toBe(true);
    expect(msg.vnav_mode).toBe(true);
    expect(msg.alt_hold).toBe(false);
    expect(msg.approach_mode).toBe(false);
    expect(msg.lnav_mode).toBe(true);
    expect(msg.tcas_operational).toBe(true);
  });

  test("adsb with position", () => {
    const msg = decode(
      "90343652300003eeda6de84f1ad2",
      new Float64Array([40.48, -3.56])
    );
    expect(msg.df).toBe("18");
    expect(msg.bds).toBe("06");
    expect(msg.latitude).toBeCloseTo(40.4749);
    expect(msg.longitude).toBeCloseTo(-3.57068);
  });
});
