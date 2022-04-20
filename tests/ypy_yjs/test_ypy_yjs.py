import asyncio

import pytest
import y_py as Y
from y_py import WebsocketProvider


@pytest.mark.asyncio
async def test_ypy_yjs(echo_server, yjs_client):
    ws_provider = WebsocketProvider("ws://localhost:1234", "my-roomname", Y.YDoc())
    ydoc = await ws_provider.connect()
    ymap = ydoc.get_map("map")
    # set a value in "inc"
    value = 1
    async with ydoc.begin_transaction() as t:
        ymap.set(t, "inc", value)
    # wait for the JS client to increment this value
    change = asyncio.Event()
    def callback(event):
        if "inc" in event.keys:
            change.set()

    ymap.observe(callback)
    await asyncio.wait_for(change.wait(), timeout=1)
    assert v_inc == value + 1
