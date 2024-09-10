# ruff: noqa: E402
# %%
import pandas as pd  # type: ignore

from rs1090 import flarm

data = pd.read_csv("../../crates/rs1090/data/flarm.csv")

# %%

decoded = flarm(
    data.rawmessage,
    data.timestamp.astype(int),
    data.sensorlatitude,
    data.sensorlongitude,
)

# %%
from itertools import batched

# 5000 is a good batch size for fast loading!
df = pd.concat(pd.DataFrame.from_records(d) for d in batched(decoded, 5000))
df = df.assign(timestamp=pd.to_datetime(df.timestamp, unit="s", utc=True))
df

# %%
# More advanced/expressive preprocessing available in the traffic library
# https://github.com/xoolive/traffic

from pitot.geodesy import distance  # type: ignore

flight = df.query(
    'icao24 == "38f27b" and '
    '"2022-06-15 07:35Z" < timestamp < "2022-06-15 09:30Z"'
).sort_values("timestamp")

flight = flight.assign(
    distance=distance(
        flight.latitude,
        flight.longitude,
        flight.reference_lat,
        flight.reference_lon,
    ),
)
# Remove noisy points (impossible distance to the receiver)
flight = flight.query("distance < 200_000")

# %%
import numpy as np

coords = flight[["timestamp", "latitude", "longitude"]]
delta = pd.concat([coords, coords.add_suffix("_1").diff()], axis=1)
delta_1 = delta.iloc[1:]
distance_nm = distance(
    (delta_1.latitude - delta_1.latitude_1).values,
    (delta_1.longitude - delta_1.longitude_1).values,
    delta_1.latitude.values,
    delta_1.longitude.values,
)
secs = delta_1.timestamp_1.dt.total_seconds()

# Remove irrealistic jumps from a point to another
flight = flight.assign(
    gs=list(np.abs(np.pad(distance_nm / secs, (1, 0), "edge")))
)


# %%
import matplotlib.pyplot as plt  # type: ignore
from cartes.crs import Lambert93, PlateCarree  # type: ignore
from cartes.osm import Overpass  # type: ignore

# Get the airport layout
airport = Overpass.request(area=dict(icao="LFMY"), aeroway=True)

fig, ax = plt.subplots(subplot_kw=dict(projection=Lambert93()))
airport.plot(
    ax,
    by="aeroway",
    runway={"lw": 2},
    aerodrome={"alpha": 0},
    radar={"alpha": 0},
)
flight.query("gs < 20000").plot(
    ax=ax, x="longitude", y="latitude", legend=False, transform=PlateCarree()
)
ax.spines["geo"].set_visible(False)
ax.yaxis.set_visible(False)
ax.set_extent((5.1, 5.16, 43.59, 43.64))  # type: ignore
fig.savefig("flarm.png")

# %%
