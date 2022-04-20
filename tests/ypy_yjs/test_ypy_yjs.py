import asyncio

import pytest
from ypy_server import YServer


@pytest.mark.asyncio
async def test_ypy_yjs(yjs_client):
    async with YServer() as yserver:
        await yserver.synced()
        v = 1
        await yserver.set_inc(v)
        v_inc = await yserver.get_inc()
        assert v_inc == v + 1
