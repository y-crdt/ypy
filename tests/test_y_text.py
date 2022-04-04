from test_helper import exchange_updates
import y_py as Y
from y_py import YText


def test_to_string():
    expected = "Hello World!"
    expected_json = '"Hello World!"'
    d = Y.YDoc()
    prelim = YText(expected)
    integrated = d.get_text("test")
    with d.begin_transaction() as txn:
        integrated.push(txn, expected)
    for test in [prelim, integrated]:
        assert str(test) == expected
        assert test.to_json() == expected_json
        assert test.__repr__() == f"YText({expected})"


def test_inserts():
    d1 = Y.YDoc()
    x = d1.get_text("test")
    with d1.begin_transaction() as txn:
        x.push(txn, "hello ")
        x.push(txn, "world!")
    value = str(x)
    expected = "hello world!"
    assert value == expected

    d2 = Y.YDoc(2)
    x = d2.get_text("test")
    exchange_updates([d1, d2])
    value = str(x)

    assert value == expected


def test_deletes():
    d1 = Y.YDoc()
    x = d1.get_text("test")

    d1.transact(lambda txn: x.push(txn, "hello world!"))

    assert len(x) == 12
    d1.transact(lambda txn: x.delete(txn, 5, 6))
    assert len(x) == 6
    d1.transact(lambda txn: x.insert(txn, 5, " Yrs"))
    assert len(x) == 10

    expected = "hello Yrs!"

    value = str(x)
    assert value == expected

    d2 = Y.YDoc(2)
    x = d2.get_text("test")

    exchange_updates([d1, d2])

    value = str(x)
    assert value == expected


def test_observer():
    d1 = Y.YDoc()

    target = None
    delta = None

    def callback(e):
        nonlocal target
        nonlocal delta
        target = e.target
        delta = e.delta

    x = d1.get_text("test")

    subscription_id = x.observe(callback)

    # insert initial data to an empty YText
    with d1.begin_transaction() as txn:
        x.insert(txn, 0, "abcd")

    assert str(target) == str(x)
    assert delta == [{"insert": "abcd"}]

    target = None
    delta = None

    # remove 2 chars from the middle
    with d1.begin_transaction() as txn:
        x.delete(txn, 1, 2)

    assert str(target) == str(x)
    assert delta == [{"retain": 1}, {"delete": 2}]
    target = None
    delta = None

    # insert item in the middle
    with d1.begin_transaction() as txn:
        x.insert(txn, 1, "e")
    assert str(target) == str(x)
    assert delta == [{"retain": 1}, {"insert": "e"}]
    target = None
    delta = None

    # free the observer and make sure that callback is no longer called
    x.unobserve(subscription_id)
    with d1.begin_transaction() as txn:
        x.insert(txn, 1, "fgh")
    assert target == None
    assert delta == None


def test_drop_sub_id():
    d = Y.YDoc()
    target = None
    delta = None

    def callback(e):
        nonlocal target
        nonlocal delta
        target = e.target
        delta = e.delta

    x = d.get_text("test")

    def register_callback(x, callback):
        # The subscription_id `i` is dropped here
        i = x.observe(callback)

    register_callback(x, callback)

    with d.begin_transaction() as txn:
        x.insert(txn, 0, "abcd")

    assert str(target) == str(x)
    assert delta == [{"insert": "abcd"}]
