import asyncio

from websockets import serve
import y_py as Y

from yutils import YMessageType, create_sync_step1_message, create_sync_step2_message, create_update_message, get_message


class YServer:
    def __init__(self):
        self.ydoc = Y.YDoc()
        self.ymap = self.ydoc.get_map("map")
        self._do_stop = asyncio.Event()
        self._stopped = asyncio.Event()
        self._synced = asyncio.Event()

    async def __aenter__(self):
        asyncio.create_task(self.serve())
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.stop()

    async def serve(self):
        async with serve(self.provider, "localhost", 1234):
            await self._do_stop.wait()
        self._stopped.set()

    async def provider(self, websocket):
        self.transaction = Transaction(self.ydoc, websocket)
        state = Y.encode_state_vector(self.ydoc)
        msg = create_sync_step1_message(state)
        await websocket.send(msg)
        while True:
            message = await websocket.recv()
            if message[0] == YMessageType.SYNC:
                message_type = message[1]
                msg = message[2:]
                if message_type == YMessageType.SYNC_STEP1:
                    state = get_message(msg)
                    update = Y.encode_state_as_update(self.ydoc, state)
                    reply = create_sync_step2_message(update)
                    await websocket.send(reply)
                elif message_type in (YMessageType.SYNC_STEP2, YMessageType.SYNC_UPDATE):
                    update = get_message(msg)
                    Y.apply_update(self.ydoc, update)
                    if message_type == YMessageType.SYNC_STEP2:
                        self._synced.set()

    async def stop(self):
        self._do_stop.set()
        await self._stopped.wait()

    async def synced(self, timeout=1):
        await asyncio.wait_for(self._synced.wait(), timeout=timeout)

    async def get_inc(self, timeout=1):
        change = asyncio.Event()
        def callback(event):
            if "inc" in event.keys:
                change.set()

        self.ymap.observe(callback)
        await asyncio.wait_for(change.wait(), timeout=timeout)
        return self.ymap["inc"]

    async def set_inc(self, value):
        async with self.transaction as t:
            self.ymap.set(t, "inc", value)


class Transaction:

    def __init__(self, ydoc, websocket):
        self.ydoc = ydoc
        self.websocket = websocket

    async def __aenter__(self):
        self.state = Y.encode_state_vector(self.ydoc)
        self.transaction_context = self.ydoc.begin_transaction()
        self.transaction = self.transaction_context.__enter__()
        return self.transaction

    async def __aexit__(self, exc_type, exc_value, exc_tb):
        res = self.transaction_context.__exit__(exc_type, exc_value, exc_tb)
        update = Y.encode_state_as_update(self.ydoc, self.state)
        message = create_update_message(update)
        await self.websocket.send(message)
        return res
