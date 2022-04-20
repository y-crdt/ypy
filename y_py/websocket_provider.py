import asyncio
from typing import Any, Dict

from websockets import connect
from .y_py import *

from .yutils import YMessageType, create_sync_step1_message, create_sync_step2_message, create_update_message, get_message


class YwsDoc:
    def __init__(self, ydoc, websocket):
        self._ydoc = ydoc
        self._websocket = websocket

    def begin_transaction(self):
        return Transaction(self._ydoc, self._websocket)

    def get_map(self, name):
        return self._ydoc.get_map(name)

class WebsocketProvider:

    def __init__(self, server_url: str, room: str, ydoc: YDoc, ws_opts: Dict[str, Any] = {}):
        self._server_url = server_url
        self._room = room
        self._ydoc = ydoc
        self._ws_opts = ws_opts

    async def __aenter__(self):
        self._connection = connect(self._server_url, **self._ws_opts)
        await self._run()
        return self._connection.__aenter__()

    async def __aexit__(self, exc_type, exc_value, exc_tb):
        self._listen_task.cancel()
        self._connection.__aexit__()

    async def connect(self):
        self._websocket = await connect(self._server_url, **self._ws_opts)
        self._ywsdoc = YwsDoc(self._ydoc, self._websocket)
        await self._run()
        return self._ywsdoc

    async def _run(self):
        state = encode_state_vector(self._ydoc)
        msg = create_sync_step1_message(state)
        await self._websocket.send(msg)
        self._listen_task = asyncio.create_task(self._listen())

    async def _listen(self):
        while True:
            message = await self._websocket.recv()
            if message[0] == YMessageType.SYNC:
                message_type = message[1]
                msg = message[2:]
                if message_type == YMessageType.SYNC_STEP1:
                    state = get_message(msg)
                    update = encode_state_as_update(self._ydoc, state)
                    reply = create_sync_step2_message(update)
                    await self._websocket.send(reply)
                elif message_type in (YMessageType.SYNC_STEP2, YMessageType.SYNC_UPDATE):
                    update = get_message(msg)
                    apply_update(self._ydoc, update)

    async def close(self):
        self._listen_task.cancel()
        await self._websocket.protocol.close()


class Transaction:

    def __init__(self, ydoc, websocket):
        self.ydoc = ydoc
        self.websocket = websocket

    async def __aenter__(self):
        self.state = encode_state_vector(self.ydoc)
        self.transaction_context = self.ydoc.begin_transaction()
        self.transaction = self.transaction_context.__enter__()
        return self.transaction

    async def __aexit__(self, exc_type, exc_value, exc_tb):
        res = self.transaction_context.__exit__(exc_type, exc_value, exc_tb)
        update = encode_state_as_update(self.ydoc, self.state)
        message = create_update_message(update)
        await self.websocket.send(message)
        return res
