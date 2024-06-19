#!/usr/bin/env python
#

import argparse
import asyncio
import json
import logging
from typing import List, Callable

import websockets

logging.basicConfig(level=logging.DEBUG, format="%(asctime)s - %(filename)s:%(lineno)s - %(message)s")
log = logging.getLogger("rs1090-ws-client")


class Jet1090WebsocketClient:
    # callbacks = {}  # f'{channel}-{event}' -> callback

    def __init__(self, websocket_url):
        self.websocket_url = websocket_url
        self._CHANNEL_CALLBACKS = {}  # f'{channel}-{event}' -> callback

    def channel_callback(self, channel: str, event: str) -> List[Callable]:
        name = (channel, event)
        return self._CHANNEL_CALLBACKS.get(name, [])

    def on(self, channel: str, event: str) -> Callable:
        def decorator(fn):
            self.register_callbacks(channel, event, fn)
            return fn

        return decorator

    def register_callbacks(self, channel: str, event: str, fn: Callable):
        name = (channel, event)
        if name not in self._CHANNEL_CALLBACKS:
            self._CHANNEL_CALLBACKS[name] = []
        self._CHANNEL_CALLBACKS[name].append(fn)
        return fn

    def connect(self):
        """connect to the websocket server, regiter callbacks before calling this"""
        loop = asyncio.get_event_loop()
        loop.run_until_complete(self._connect())

    async def _connect(self):
        # ping/pong keepalive
        # disabled: https://websockets.readthedocs.io/en/stable/topics/timeouts.html
        self._connection = await websockets.connect(self.websocket_url, ping_interval=None)
        log.info("connected to %s", self.websocket_url)
        for (channel, event), callbacks in self._CHANNEL_CALLBACKS.items():
            if channel == "phoenix" and event == "heartbeat":
                continue
            await self._join(channel)

    def listen(self) -> None:
        loop = asyncio.get_event_loop()
        loop.run_until_complete(asyncio.gather(self._heartbeat(), self._dispatch()))

    async def _join(self, channel: str) -> None:
        await self._connection.send(json.dumps(["0", "0", channel, "phx_join", {}]))

    async def _heartbeat(self):
        while True:
            await self._connection.send(json.dumps(["0", "0", "phoenix", "heartbeat", {}]))
            log.info("heartbeat sent")

            await asyncio.sleep(15)

    async def _dispatch(self):
        """dispatch messages to registered callbacks"""
        async for message in self._connection:
            # log.info("message: %s", message)
            [join_ref, ref, channel, event, payload] = json.loads(message)
            status, response = payload["status"], payload["response"]
            # {'status': status, 'response': response} = payload
            name = (channel, event)
            if name in self._CHANNEL_CALLBACKS:
                for fn in self._CHANNEL_CALLBACKS[name]:
                    fn(join_ref, ref, channel, event, status, response)


def on_joining_system(_join_ref, _ref, channel, event, status, response) -> None:  # noqa
    log.info("joined %s/%s, status: %s, response: %s", channel, event, status, response)


def on_heartbeat(join_ref, ref, channel, event, status, response) -> None:  # noqa
    log.info("heartbeat: %s", response)


def on_datetime(join_ref, ref, channel, event, status, response) -> None:  # noqa
    log.info("datetime: %s", response)


def on_jet1090_message(join_ref, ref, channel, event, status, response) -> None:  # noqa
    log.info("jet1090 message: %s", response)


def main(ws_url):
    client = Jet1090WebsocketClient(ws_url)
    client.register_callbacks("phoenix", "heartbeat", on_heartbeat)
    client.register_callbacks("system", "phx_join", on_joining_system)
    client.register_callbacks("system", "datetime", on_datetime)
    client.register_callbacks("jet1090", "data", on_jet1090_message)
    client.connect()
    client.listen()


if __name__ == "__main__":
    default_websocket_url = "ws://127.0.0.1:5000/websocket"

    parser = argparse.ArgumentParser()
    parser.add_argument("-u", "--websocket-url", dest="websocket_url", type=str, default=default_websocket_url)
    parser.add_argument(
        "-l", "--log-level", dest="log_level", default="info", choices=["debug", "info", "warning", "error", "critical"]
    )
    args = parser.parse_args()
    log.setLevel(args.log_level.upper())
    print(log)

    try:
        main(args.websocket_url)
    except KeyboardInterrupt:
        print("\rbye.")
