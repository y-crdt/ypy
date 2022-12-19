from y_py import YDoc, AfterTransactionEvent

import y_py as Y
import pytest


def test_constructor_options():
    # Ensure that valid versions can be called without error
    YDoc()
    with_id = YDoc(1)
    assert with_id.client_id == 1
    YDoc(1, "utf-8", True)
    YDoc(client_id=2, offset_kind="utf-8", skip_gc=True)
    YDoc(client_id=3)
    YDoc(4, skip_gc=True)

    # Handle encoding string variation
    YDoc(offset_kind="utf8")
    YDoc(offset_kind="utf-8")
    YDoc(offset_kind="UTF-8")
    YDoc(offset_kind="UTF32")

    # Ensure that incorrect encodings throw error
    with pytest.raises(ValueError):
        YDoc(offset_kind="UTF-0xDEADBEEF")
    with pytest.raises(ValueError):
        YDoc(offset_kind="😬")


def test_encoding():
    """
    Tests encoding / decoding all primitive data types in an array.
    """
    doc = YDoc()
    receiver = YDoc()
    array = doc.get_array("test")
    contents = [True, 42, "string"]
    with doc.begin_transaction() as txn:
        array.insert_range(txn, 0, contents)

    state_vec = Y.encode_state_vector(receiver)
    update = Y.encode_state_as_update(doc, state_vec)
    Y.apply_update(receiver, update)
    value = list(receiver.get_array("test"))
    assert value == contents


def test_boolean_encoding():
    """
    Makes sure the boolean types are preserved.
    Added due to bug where bools turn to ints during encoding /decoding.
    """
    doc = YDoc()
    receiver = YDoc()
    array = doc.get_array("test")
    with doc.begin_transaction() as txn:
        array.insert(txn, 0, True)

    state_vec = Y.encode_state_vector(receiver)
    update = Y.encode_state_as_update(doc, state_vec)
    Y.apply_update(receiver, update)
    value = list(receiver.get_array("test"))
    assert type(value[0]) == type(True)


def test_tutorial():
    d1 = Y.YDoc()
    text = d1.get_text("test")
    with d1.begin_transaction() as txn:
        text.extend(txn, "hello world!")

    d2 = Y.YDoc()
    state_vector = Y.encode_state_vector(d2)
    diff = Y.encode_state_as_update(d1, state_vector)
    Y.apply_update(d2, diff)

    value = str(d2.get_text("test"))

    assert value == "hello world!"


def test_observe_after_transaction():
    doc = Y.YDoc()
    text = doc.get_text("test")
    before_state = None
    after_state = None
    delete_set = None

    def callback(event):
        nonlocal before_state
        nonlocal after_state
        nonlocal delete_set
        before_state = event.before_state
        after_state = event.after_state
        delete_set = event.delete_set

    # Subscribe callback
    sub = doc.observe_after_transaction(callback)

    # Update the document
    with doc.begin_transaction() as txn:
        text.insert(txn, 0, "abc")
        text.delete_range(txn, 1, 2)

    assert before_state != None
    assert after_state != None
    assert delete_set != None


def test_get_update():
    """
    Ensures that developers can access the encoded update data in the `observe_after_transaction` event.
    """
    d = Y.YDoc()
    m = d.get_map("foo")
    r = d.get_map("foo")
    update: bytes = None

    def get_update(event: AfterTransactionEvent) -> None:
        nonlocal update
        update = event.get_update()

    d.observe_after_transaction(get_update)

    with d.begin_transaction() as txn:
        m.set(txn, "hi", "there")

    assert type(update) == bytes
