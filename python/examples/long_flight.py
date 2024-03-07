# ruff: noqa: E402
# %%
import pandas as pd
from rs1090 import decode

data = pd.read_csv(
    "../../crates/rs1090/data/long_flight.csv",
    names=["timestamp", "rawmsg"],
)

# %%
decoded = decode(data.rawmsg.str[18:], data.timestamp)
df = pd.json_normalize(decoded)
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
