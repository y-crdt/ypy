import types
from utils import read_config
import y_py as Y

doc = Y.YDoc()


def send_update(writer):
    state_vector = Y.encode_state_vector(doc)
    writer.write(state_vector)
    receive(update)
    Y.apply_update(doc, update)


def receive_update(reader):
    sv = reader.read()
    update = Y.encode_state_as_update(doc, sv)
    send(update)


def setup_stream():
    pass


def main():
    doc.observe("update", send_update)
    setup_stream()
