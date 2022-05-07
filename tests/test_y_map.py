import pytest
import y_py as Y
from y_py import YMap


def test_get():
    d = Y.YDoc()
    m = d.get_map("map")

    # Put user info into the map.
    with d.begin_transaction() as txn:
        m.update(txn, {"username": "John", "online": True})

    # Extract the information from the map.
    assert m.get("username") == "John"
    assert m["online"] == True
    # Ensure the default value is returned when key doesn't exist in map.
    assert m.get("secretIdentity", "basic") == "basic"
    # Ensure that nonexistant keys without default values return None.
    assert m.get("SocialSecurityNumber") is None
    # Ensure that indexing a non_existant key with bracket notation produces `KeyError`
    with pytest.raises(KeyError):
        m["doesn't exist"]


def test_set():
    d1 = Y.YDoc()
    x = d1.get_map("test")

    value = x.get("key")
    assert value == None

    d1.transact(lambda txn: x.set(txn, "key", "value1"))
    value = x["key"]
    assert value == "value1"

    d1.transact(lambda txn: x.set(txn, "key", "value2"))
    value = x["key"]
    assert value == "value2"


def test_update():
    doc = Y.YDoc()
    ymap = doc.get_map("dict")
    dict_vals = {"user_id": 1, "username": "Josh", "is_active": True}
    tuple_vals = ((k, v) for k, v in dict_vals.items())

    # Test updating with a dictionary
    with doc.begin_transaction() as txn:
        ymap.update(txn, dict_vals)
    assert ymap.to_json() == dict_vals

    # Test updating with an iterator
    ymap = doc.get_map("tuples")
    with doc.begin_transaction() as txn:
        ymap.update(txn, tuple_vals)
    assert ymap.to_json() == dict_vals

    # Test non-string key error
    with pytest.raises(Exception) as e:
        with doc.begin_transaction() as txn:
            ymap.update(txn, [(1, 2)])

    # Test non-kv-tuple error
    with pytest.raises(Exception) as e:
        with doc.begin_transaction() as txn:
            ymap.update(txn, [1])


def test_set_nested():
    d1 = Y.YDoc()
    x = d1.get_map("test")
    nested = Y.YMap({"a": "A"})

    # check out to_json(), setting a nested map in set(), adding to an integrated value

    d1.transact(lambda txn: x.set(txn, "key", nested))
    d1.transact(lambda txn: nested.set(txn, "b", "B"))

    json = x.to_json()
    assert json == {"key": {"a": "A", "b": "B"}}


def test_pop():
    d1 = Y.YDoc()
    x = d1.get_map("test")

    d1.transact(lambda txn: x.set(txn, "key", "value1"))
    length = len(x)
    value = x["key"]
    assert length == 1
    assert value == "value1"
    d1.transact(lambda txn: x.pop(txn, "key"))

    with pytest.raises(KeyError):
        with d1.begin_transaction() as txn:
            x.pop(txn, "does not exist")
        assert x.pop(txn, "does not exist", "fallback") == "fallback"
    length = len(x)
    value = x.get("key")
    assert length == 0
    assert value == None

    d1.transact(lambda txn: x.set(txn, "key", "value2"))
    length = len(x)
    value = x["key"]
    assert length == 1
    assert value == "value2"


def test_iterator():
    d = Y.YDoc()
    x = d.get_map("test")

    with d.begin_transaction() as txn:
        x.set(txn, "a", 1)
        x.set(txn, "b", 2)
        x.set(txn, "c", 3)
        expected = {"a": 1, "b": 2, "c": 3}
        for (key, val) in x.items():
            v = expected[key]
            assert val == v
            del expected[key]

        expected = {"a": 1, "b": 2, "c": 3}
        for key in x:
            assert key in expected
            assert key in x


def test_observer():
    d1 = Y.YDoc()
    x = d1.get_map("test")
    target = None
    entries = None

    def get_value(x):
        return x.to_json()

    def callback(e):
        nonlocal target
        nonlocal entries
        target = e.target
        entries = e.keys

    subscription_id = x.observe(callback)

    # insert initial data to an empty YMap
    with d1.begin_transaction() as txn:
        x.set(txn, "key1", "value1")
        x.set(txn, "key2", 2)

    assert get_value(target) == get_value(x)
    assert entries == {
        "key1": {"action": "add", "newValue": "value1"},
        "key2": {"action": "add", "newValue": 2},
    }

    target = None
    entries = None

    # remove an entry and update another on
    with d1.begin_transaction() as txn:
        x.pop(txn, "key1")
        x.set(txn, "key2", "value2")

    assert get_value(target) == get_value(x)
    assert entries == {
        "key1": {"action": "delete", "oldValue": "value1"},
        "key2": {"action": "update", "oldValue": 2, "newValue": "value2"},
    }

    target = None
    entries = None

    # free the observer and make sure that callback is no longer called
    x.unobserve(subscription_id)
    with d1.begin_transaction() as txn:
        x.set(txn, "key1", [6])
    assert target == None
    assert entries == None


def test_deep_observe():
    """
    Ensure that changes to elements inside the array trigger a callback.
    """
    doc = Y.YDoc()
    container = doc.get_map("container")
    inner_map = Y.YMap({"key": "initial"})
    with doc.begin_transaction() as txn:
        container.set(txn, "inner", inner_map)

    events = None

    def callback(e: list):
        nonlocal events
        events = e

    sub = container.observe_deep(callback)
    with doc.begin_transaction() as txn:
        container["inner"].set(txn, "addition", 1)

    events = None
    container.unobserve(sub)
    with doc.begin_transaction() as txn:
        container["inner"].set(txn, "don't show up", 1)

    assert events is None
