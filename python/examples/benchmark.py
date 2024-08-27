# ruff: noqa: E402
# %%
import bench_pms  # type: ignore
import pandas as pd  # type: ignore
from pyModeS import c_common, py_common  # type: ignore

from rs1090 import decode

msg = "8DA05F219B06B6AF189400CBC33F"

decode(msg)
bench_pms.decode(msg, py_common)
bench_pms.decode(msg, c_common)

data = pd.read_csv(
    "../../crates/rs1090/data/long_flight.csv",
    names=["timestamp", "rawmsg"],
)

# %%
# %%timeit
# 1.69 s ± 60.7 ms per loop (mean ± std. dev. of 7 runs, 1 loop each)
decoded = decode(data.rawmsg.str[18:], data.timestamp, reference=(43.3, 1.35))

# %%
# %%timeit
# 1.46 s ± 30.3 ms per loop (mean ± std. dev. of 7 runs, 1 loop each)
decoded = decode(data.rawmsg.str[18:])

# %%
# %%timeit
# 4.15 s ± 92.8 ms per loop (mean ± std. dev. of 7 runs, 1 loop each)
decoded = decode(data.rawmsg.str[18:], batch=data.shape[0])

# %%
# %%timeit
# 9.27 s ± 154 ms per loop (mean ± std. dev. of 7 runs, 1 loop each)
decoded = [bench_pms.decode(msg, c_common) for msg in data.rawmsg.str[18:]]

# %%
# %%timeit
# 16 s ± 183 ms per loop (mean ± std. dev. of 7 runs, 1 loop each)
decoded = [bench_pms.decode(msg, py_common) for msg in data.rawmsg.str[18:]]


# %%
# cargo bench:  time:   [741.01 ms 755.14 ms 770.26 ms]

# %%
n = data.shape[0]
result = pd.DataFrame.from_records(
    [
        {"type": "rs1090 (Rust, default)", "time": 0.755, "std": 0.015},
        {"type": "rs1090 (Python, default)", "time": 1.46, "std": 0.03},
        {"type": "rs1090 (Python, single core)", "time": 4.15, "std": 0.093},
        {"type": "pyModeS (compiled)", "time": 9.27, "std": 0.154},
        {"type": "pyModeS (Python)", "time": 16, "std": 0.183},
    ]
)

# %%

import altair as alt  # type: ignore

chart = (
    alt.Chart(result)
    .mark_bar()
    .encode(
        alt.X(
            "time",
            axis=alt.Axis(format="~s"),
            title="Decoding time (in μs per message)",
        ),
        alt.Y("type", title=None, sort="-x"),
        alt.Color("type", legend=None),  # type: ignore
    )
    .transform_calculate(time=f"datum.time / {n}")
    .properties(width=500, height=100)
    .configure_axis(
        labelFont="Noto Sans",
        titleFont="Noto Sans",
        labelFontSize=13,
        titleFontSize=15,
        titleAnchor="end",
    )
)
chart.save("benchmark.svg")
chart

# %%
