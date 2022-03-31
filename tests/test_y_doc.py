from y_py import YDoc
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
        array.insert(txn, 0, contents)

    state_vec = Y.encode_state_vector(receiver)
    update = Y.encode_state_as_update(doc, state_vec)
    Y.apply_update(receiver, update)
    value = receiver.get_array("test").to_json()
    assert value == contents


def test_boolean_encoding():
    """
    Makes sure the boolean types are preserved.
    Added due to bug where bools turn to ints during encoding /decoding.
    """
    doc = YDoc()
    receiver = YDoc()
    array = doc.get_array("test")
    contents = [True]
    with doc.begin_transaction() as txn:
        array.insert(txn, 0, contents)

    state_vec = Y.encode_state_vector(receiver)
    update = Y.encode_state_as_update(doc, state_vec)
    Y.apply_update(receiver, update)
    value = receiver.get_array("test").to_json()
    assert type(value[0]) == type(contents[0])


def test_tutorial():
    d1 = Y.YDoc()
    text = d1.get_text("test")
    with d1.begin_transaction() as txn:
        text.push(txn, "hello world!")

    d2 = Y.YDoc()
    state_vector = Y.encode_state_vector(d2)
    diff = Y.encode_state_as_update(d1, state_vector)
    Y.apply_update(d2, diff)

    value = str(d2.get_text("test"))

    assert value == "hello world!"
