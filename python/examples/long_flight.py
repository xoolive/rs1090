# ruff: noqa: E402
# %%
import pandas as pd  # type: ignore

from rs1090 import decode

data = pd.read_csv(
    "../../crates/rs1090/data/long_flight.csv",
    names=["timestamp", "rawmsg"],
)

# %%
decoded = decode(data.rawmsg.str[18:], data.timestamp, reference=(43.3, 1.35))

# %%
df = pd.DataFrame.from_records(decoded)
df = df.assign(timestamp=pd.to_datetime(df.timestamp, unit="s", utc=True))
df

# %%
from itertools import batched

# 5000 is quite fastest actually!
df = pd.concat(pd.DataFrame.from_records(d) for d in batched(decoded, 5000))
df = df.assign(timestamp=pd.to_datetime(df.timestamp, unit="s", utc=True))
df

# %%
from traffic.core import Traffic  # type: ignore

t = Traffic(df)
t
# %%
t["3944ed"].map_leaflet()
# %%
t["486257"].map_leaflet()
# %%
