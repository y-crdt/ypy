from test_helper import exchange_updates
import pytest

from y_py import YDoc, YArray


def test_inserts():
    d1 = YDoc(1)
    x = d1.get_array("test")

    with d1.begin_transaction() as txn:
        x.insert(txn, 0, [1, 2.5, "hello", ["world"], True])

    with d1.begin_transaction() as txn:
        x.push(txn, [{"key": "value"}])

    expected = [1, 2.5, "hello", ["world"], True, {"key": "value"}]

    value = x.to_json()
    assert value == expected

    d2 = YDoc(2)
    x = d2.get_array("test")

    exchange_updates([d1, d2])

    value = x.to_json()
    assert value == expected


def test_to_string():
    arr = YArray([7, "awesome", True, ["nested"], {"testing": "dicts"}])
    expected_str = "[7, 'awesome', True, ['nested'], {'testing': 'dicts'}]"
    assert str(arr) == expected_str
    assert arr.__repr__() == f"YArray({expected_str})"


def test_inserts_nested():
    d1 = YDoc()
    x = d1.get_array("test")

    nested = YArray()
    d1.transact(lambda txn: nested.push(txn, ["world"]))
    d1.transact(lambda txn: x.insert(txn, 0, [1, 2, nested, 3, 4]))
    d1.transact(lambda txn: nested.insert(txn, 0, ["hello"]))

    expected = [1, 2, ["hello", "world"], 3, 4]

    value = d1.transact(lambda txn: x.to_json())
    assert value == expected

    d2 = YDoc()
    x = d2.get_array("test")

    exchange_updates([d1, d2])

    value = x.to_json()
    assert value == expected


def test_delete():
    d1 = YDoc(1)
    assert d1.client_id == 1
    x = d1.get_array("test")

    d1.transact(lambda txn: x.insert(txn, 0, [1, 2, ["hello", "world"], True]))
    d1.transact(lambda txn: x.delete(txn, 1, 2))

    expected = [1, True]

    value = x.to_json()
    assert value == expected

    d2 = YDoc(2)
    x = d2.get_array("test")

    exchange_updates([d1, d2])

    value = x.to_json()
    assert value == expected


def test_get():
    d1 = YDoc()
    integrated = d1.get_array("test")
    prelim = YArray()

    d1.transact(lambda txn: integrated.insert(txn, 0, [1, 2, True]))
    d1.transact(lambda txn: integrated.insert(txn, 1, ["hello", "world"]))

    expected = [1, "hello", "world", 2, True]
    prelim = YArray(expected)

    for arr in [integrated, prelim]:
        # Forward indexing
        for i, expected_value in enumerate(expected):
            assert arr[i] == expected_value

        with pytest.raises(IndexError):
            arr[5]

        # Negative indexing
        for i, expected_value in enumerate(reversed(expected)):
            index = -(i + 1)
            assert arr[index] == expected_value

        with pytest.raises(IndexError):
            arr[-6]

        # Slices
        assert arr[0:] == expected
        assert arr[4:1:-1] == expected[4:1:-1]
        assert arr[::-1] == expected[::-1]


def test_iterator():
    d1 = YDoc()
    x = d1.get_array("test")

    with d1.begin_transaction() as txn:
        x.insert(txn, 0, [1, 2, 3])
    assert len(x) == 3
    i = 1
    # Test iteration
    for v in x:
        assert v == i
        i += 1
    # Test contains
    assert 2 in x


def test_borrow_mut_edge_case():
    """
    Tests for incorrect overlap in successive mutable borrows of YTransaction and YArray.
    """
    doc = YDoc()
    arr = doc.get_array("test")
    with doc.begin_transaction() as txn:
        arr.insert(txn, 0, [1, 2, 3])

    # Ensure multiple transactions can be called in a row with the same variable name `txn`
    with doc.begin_transaction() as txn:
        # Ensure that multiple mutable borrow functions can be called in a tight loop
        for i in range(2000):
            arr.insert(txn, 2, [1, 2, 3])
            arr.delete(txn, 0, 3)


def test_observer():
    d1 = YDoc()

    x = d1.get_array("test")

    target = None
    delta = None

    def callback(e):
        nonlocal target
        nonlocal delta
        target = e.target
        delta = e.delta

    subscription_id = x.observe(callback)

    # insert initial data to an empty YArray
    with d1.begin_transaction() as txn:
        x.insert(txn, 0, [1, 2, 3, 4])
    assert target.to_json() == x.to_json()
    assert delta == [{"insert": [1, 2, 3, 4]}]

    target = None
    delta = None

    # remove 2 items from the middle
    with d1.begin_transaction() as txn:
        x.delete(txn, 1, 2)
    assert target.to_json() == x.to_json()
    assert delta == [{"retain": 1}, {"delete": 2}]

    target = None
    delta = None

    # insert  item in the middle
    with d1.begin_transaction() as txn:
        x.insert(txn, 1, [5])
    assert target.to_json() == x.to_json()
    assert delta == [{"retain": 1}, {"insert": [5]}]

    target = None
    delta = None

    # Cancel the observer and make sure that callback is no longer called
    x.unobserve(subscription_id)

    with d1.begin_transaction() as txn:
        x.insert(txn, 1, [6])

    assert target == None
    assert delta == None
