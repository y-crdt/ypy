import pytest
from test_helper import exchange_updates

import y_py as Y


def test_inserts():
    d1 = Y.YDoc()
    x = d1.get_text("test")
    with d1.begin_transaction() as txn:
        x.push(txn, "hello ")
        x.push(txn, "world!")
        value = x.to_string(txn)
    expected = "hello world!"
    assert value == expected

    d2 = Y.YDoc(2)
    x = d2.get_text("test")

    exchange_updates([d1, d2])
    with d2.begin_transaction() as txn:
        value = x.to_string(txn)

    assert value == expected


def test_deletes():
    d1 = Y.YDoc()
    x = d1.get_text("test")

    d1.transact(lambda txn: x.push(txn, "hello world!"))

    assert x.length == 12
    d1.transact(lambda txn: x.delete(txn, 5, 6))
    assert x.length == 6
    d1.transact(lambda txn: x.insert(txn, 5, " Yrs"))
    assert x.length == 10

    expected = "hello Yrs!"

    value = d1.transact(lambda txn: x.to_string(txn))
    assert value == expected

    d2 = Y.YDoc(2)
    x = d2.get_text("test")

    exchange_updates([d1, d2])

    value = d2.transact(lambda txn: x.to_string(txn))
    assert value == expected


def test_observer():
    d1 = Y.YDoc()

    def get_value(x):
        with d1.begin_transaction() as txn:
            return x.to_string(txn)

    target = None
    delta = None

    def callback(e):
        nonlocal target
        nonlocal delta
        target = e.target
        delta = e.delta

    x = d1.get_text("test")

    observer = x.observe(callback)

    # insert initial data to an empty YText
    with d1.begin_transaction() as txn:
        x.insert(txn, 0, "abcd")

    assert get_value(target) == get_value(x)
    assert delta == [{"insert": "abcd"}]

    target = None
    delta = None

    # remove 2 chars from the middle
    with d1.begin_transaction() as txn:
        x.delete(txn, 1, 2)

    assert get_value(target) == get_value(x)
    assert delta == [{"retain": 1}, {"delete": 2}]
    target = None
    delta = None

    # insert item in the middle
    with d1.begin_transaction() as txn:
        x.insert(txn, 1, "e")
    assert get_value(target) == get_value(x)
    assert delta == [{"retain": 1}, {"insert": "e"}]
    target = None
    delta = None

    # free the observer and make sure that callback is no longer called
    del observer
    with d1.begin_transaction() as txn:
        x.insert(txn, 1, "fgh")
    assert target == None
    assert delta == None
