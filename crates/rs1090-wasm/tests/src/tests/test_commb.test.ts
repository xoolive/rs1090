import { decode } from "rs1090-wasm";
import { expect, describe, test } from "@jest/globals";

describe("CommB Decoding Tests", () => {
  test("bds20", () => {
    {
      const msg = decode("A000083E202CC371C31DE0AA1CCF");
      expect(msg.df).toBe("20");
      const bds20 = msg.bds20;
      expect(bds20).not.toBeNull();
      expect(bds20.callsign).toBe("KLM1017");
    }
    {
      const msg = decode("A0001993202422F2E37CE038738E");
      expect(msg.df).toBe("20");
      const bds20 = msg.bds20;
      expect(bds20).not.toBeNull();
      expect(bds20.callsign).toBe("IBK2873");
    }
  });

  test("bds40", () => {
    const msg = decode("A000029C85E42F313000007047D3");
    expect(msg.df).toBe("20");
    const bds40 = msg.bds40;
    expect(bds40).not.toBeNull();
    expect(bds40.selected_mcp).toBe(3000);
    expect(bds40.selected_fms).toBe(3000);
    expect(bds40.barometric_setting).toBe(1020);
  });

  test("bds50", () => {
    const msg = decode("A000139381951536E024D4CCF6B5");
    expect(msg.df).toBe("20");
    const bds50 = msg.bds50;
    expect(bds50).not.toBeNull();
    expect(bds50.roll).toBeCloseTo(2.11, 2);
    expect(bds50.track).toBeCloseTo(114.2578);
    expect(bds50.groundspeed).toBe(438);
    expect(bds50.track_rate).toBe(0.125);
    expect(bds50.TAS).toBe(424);
  });

  test("bds60", () => {
    const msg = decode("A00004128F39F91A7E27C46ADC21");
    expect(msg.df).toBe("20");
    const bds60 = msg.bds60;
    expect(bds60).not.toBeNull();
    expect(bds60.heading).toBeCloseTo(42.71484);
  });
});
