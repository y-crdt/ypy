from turtle import position
from p5 import *
from y_py import YDoc, YArray, AfterTransactionEvent
from client import YDocWSClient

doc: YDoc
strokes: YArray
client: YDocWSClient


def setup():
    """
    Initialization logic that runs before the `draw()` loop.
    """
    global strokes
    global doc
    global client
    title("Ypy Drawing Demo")
    size(720, 480)
    doc = YDoc(0)
    strokes = doc.get_array("strokes")
    client = YDocWSClient()
    doc.observe_after_transaction(client.send_updates)
    


def draw():
    """
    Handles user input and updates the canvas.
    """
    global strokes
    global doc
    global client
    client.apply_updates(doc)
    rect_mode(CENTER)
    background(255)
    if mouse_is_pressed:
        with doc.begin_transaction() as txn:
            strokes.append(txn, [mouse_x, mouse_y])
    fill(0)
    no_stroke()
    for x,y in strokes:
        ellipse((x, y), 33, 33)

run(frame_rate=60)
