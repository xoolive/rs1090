#!/usr/bin/env python
#

from __future__ import annotations

import argparse
import asyncio
import json
import logging
from typing import Callable, Set

import websockets

logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(filename)s:%(lineno)s - %(message)s")
log = logging.getLogger("rs1090-ws-client")


class Channel:
    def __init__(self, connection: Jet1090WebsocketClient, channel: str, loop=None) -> None:
        self.connection = connection
        self.channel = channel
        self.loop = loop or asyncio.get_event_loop()
        self._event_handlers: dict[str, Set[Callable]] = {}
        self.join_ref = 0
        self.ref = 0

    def join(self) -> Channel:
        return self.send("phx_join", {})

    def run_event_handler(self, event: str, *args, **kwargs) -> None:
        """this is called from the connection when a message is received"""
        if event == "phx_reply":
            log.info("ignore message: %s %s %s", event, args, kwargs)
            return

        for fn in self._event_handlers.get(event, []):
            fn(*args, **kwargs)

    def send(self, event: str, payload: dict) -> Channel:
        message = json.dumps(["0", "0", self.channel, event, payload])
        self.loop.run_until_complete(self.connection.send(message))
        if event == "phx_join":
            self.join_ref += 1
        self.ref += 1
        return self

    def on_event(self, event: str, fn: Callable) -> Channel:
        if event not in self._event_handlers:
            self._event_handlers[event] = set()
        self._event_handlers[event].add(fn)
        return self

    def off_event(self, event: str, fn: Callable) -> Channel:
        self._event_handlers[event].remove(fn)
        return self

    def on(self, event: str) -> Callable:
        def decorator(fn):
            self.on_event(event, fn)
            return fn

        return decorator

    def off(self, event: str) -> Callable:
        def decorator(fn):
            self.off_event(event, fn)

        return decorator


class Jet1090WebsocketClient:
    # callbacks = {}  # f'{channel}-{event}' -> callback

    def __init__(self, websocket_url):
        self.websocket_url = websocket_url
        self._CHANNEL_CALLBACKS = {}  # f'{channel}-{event}' -> callback
        self.channels = {}
        self.loop = asyncio.get_event_loop()

    def connect(self):
        """connect to the websocket server, regiter callbacks before calling this"""
        self.loop.run_until_complete(self._connect())

    async def _connect(self):
        # ping/pong keepalive
        # disabled: https://websockets.readthedocs.io/en/stable/topics/timeouts.html
        self._connection = await websockets.connect(self.websocket_url, ping_interval=None)
        log.info("connected to %s", self.websocket_url)

    def add_channel(self, channel: str) -> Channel:
        ch = Channel(self, channel, self.loop)
        self.channels[channel] = ch
        return ch

    async def send(self, message: str):
        await self._connection.send(message)

    def start(self) -> None:
        loop = asyncio.get_event_loop()
        loop.run_until_complete(asyncio.gather(self._heartbeat(), self._dispatch()))

    async def _heartbeat(self):
        ref = 0
        while True:
            await self._connection.send(json.dumps(["0", str(ref), "phoenix", "heartbeat", {}]))
            log.info("heartbeat sent")

            await asyncio.sleep(60)
            ref += 1

    async def _dispatch(self):
        """dispatch messages to registered callbacks"""
        async for message in self._connection:
            log.debug("message: %s", message)
            [join_ref, ref, channel, event, payload] = json.loads(message)
            status, response = payload["status"], payload["response"]
            ch: Channel | None = self.channels.get(channel)
            if ch:
                ch.run_event_handler(event, join_ref, ref, channel, event, status, response)


def on_joining_system(_join_ref, _ref, channel, event, status, response) -> None:  # noqa
    log.info("joined %s/%s, status: %s, response: %s", channel, event, status, response)


def on_heartbeat(join_ref, ref, channel, event, status, response) -> None:  # noqa
    log.info("heartbeat: %s", response)


def on_datetime(join_ref, ref, channel, event, status, response) -> None:  # noqa
    # log.info("datetime: %s", response)
    pass


def on_jet1090_message(join_ref, ref, channel, event, status, response) -> None:  # noqa
    skipped_fields = ["timestamp", "timesource", "system", "frame"]
    log.info("jet1090: %s", {k: v for k, v in response["timed_message"].items() if k not in skipped_fields})


def main(ws_url):
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)

    client = Jet1090WebsocketClient(ws_url)
    client.connect()

    system_channel = client.add_channel("system").on_event("datetime", on_datetime)
    system_channel.join()
    # client.add_channel("system").join()

    jet1090_channel = client.add_channel("jet1090").on_event("data", on_jet1090_message)
    jet1090_channel.join()

    client.start()


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
