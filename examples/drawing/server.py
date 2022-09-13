import asyncio
from turtle import update
import websockets

connected = set()

async def server_handler(websocket):
    # Register.
    connected.add(websocket)
    try:
        async for message in websocket:
            peers = {peer for peer in connected if peer is not websocket}
            websockets.broadcast(peers, message)

    except websockets.exceptions.ConnectionClosedError: 
        pass
    finally:
        # Unregister.
        connected.remove(websocket)


async def main():
    async with websockets.serve(server_handler, "localhost", 7654):
        await asyncio.Future()  # run forever

if __name__ == "__main__":
    asyncio.run(main())
