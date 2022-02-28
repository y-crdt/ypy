import asyncio
import websockets
import y_py as Y


async def hello(websocket):
    name = await websocket.recv()
    print(f"<<< {name}")

    greeting = f"Hello {name}!"

    await websocket.send(greeting)
    print(f">>> {greeting}")


async def main():
    async with websockets.serve(hello, "localhost", 8765):
        await asyncio.Future()  # run forever


if __name__ == "__main__":
    asyncio.run(main())


def tut():
    d1 = Y.YDoc()
    # Create a new YText object in the YDoc
    text = d1.get_text("test")
    # Start a transaction in order to update the text
    with d1.begin_transaction() as txn:
        # Add text contents
        text.push(txn, "hello world!")

    # Create another document
    d2 = Y.YDoc()
    # Share state with the original document
    state_vector = Y.encode_state_vector(d2)
    diff = Y.encode_state_as_update(d1, state_vector)
    Y.apply_update(d2, diff)

    with d2.begin_transaction() as txn:
        value = d2.get_text("test").to_string(txn)
