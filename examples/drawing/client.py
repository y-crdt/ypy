import asyncio
from time import sleep
import websockets
import y_py as Y
import queue
import concurrent.futures
import threading

# Code based on the [`websockets` patter documentation](https://websockets.readthedocs.io/en/stable/howto/patterns.html)

class YDocWSClient:

    def __init__(self, uri = "ws://localhost:7654"):
        self.send_q = queue.Queue()
        self.recv_q = queue.Queue()
        self.uri = uri
        def async_loop():
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)

            loop.run_until_complete(self.start_ws_client())
            loop.close()
        ws_thread = threading.Thread(target=async_loop, daemon=True)
        ws_thread.start()

    def send_updates(self, txn_event: Y.AfterTransactionEvent):
        update = txn_event.get_update()
        # Sometimes transactions don't write, which means updates are empty.
        # We only care about updates with meaningful mutations.
        if update != b'\x00\x00':
            self.send_q.put_nowait(update)

    def apply_updates(self, doc: Y.YDoc):
        while not self.recv_q.empty():
            update = self.recv_q.get_nowait()
            Y.apply_update(doc, update)
    
    async def client_handler(self, websocket):
        consumer_task = asyncio.create_task(self.consumer_handler(websocket))
        producer_task = asyncio.create_task(self.producer_handler(websocket))
        done, pending = await asyncio.wait(
            [consumer_task, producer_task],
            return_when=asyncio.FIRST_COMPLETED,
        )
        for task in pending:
            task.cancel()

    async def consumer_handler(self, websocket):
        async for message in websocket:
            self.recv_q.put_nowait(message)
    
    async def producer_handler(self, websocket):
        loop = asyncio.get_running_loop()
        while True:
            update = await loop.run_in_executor(None,self.send_q.get)
            await websocket.send(update)

    async def start_ws_client(self):
        async with websockets.connect(self.uri) as websocket:
            await self.client_handler(websocket)
    