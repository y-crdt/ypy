from copy import deepcopy
import json
import pytest
import y_py as Y
from y_py import YMap, YMapEvent


def test_get():
    import y_py as Y

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


def test_to_json():
    contents = {"emojis": [
        {"icon":"👍", "description": "thumbs up", "positive":True},
        {"icon":"👎", "description": "thumbs down", "positive":False},
        ]}
    doc = Y.YDoc()
    prelim = Y.YMap(deepcopy(contents))
    integrated = doc.get_map("map")
    with doc.begin_transaction() as txn:
        integrated.update(txn, contents)
    # ensure that it works with python json
    assert json.loads(integrated.to_json()) == contents


def test_update():
    doc = Y.YDoc()
    ymap = doc.get_map("dict")
    dict_vals = {"user_id": 1, "username": "Josh", "is_active": True}
    tuple_vals = ((k, v) for k, v in dict_vals.items())

    # Test updating with a dictionary
    with doc.begin_transaction() as txn:
        ymap.update(txn, dict_vals)
    assert dict(ymap) == dict_vals

    # Test updating with an iterator
    ymap = doc.get_map("tuples")
    with doc.begin_transaction() as txn:
        ymap.update(txn, tuple_vals)
    assert dict(ymap) == dict_vals

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

    d1.transact(lambda txn: x.set(txn, "key", nested))
    d1.transact(lambda txn: nested.set(txn, "b", "B"))

    assert type(x["key"]) == Y.YMap
    assert {k : dict(v) for k, v in x.items()} == {"key": {"a": "A", "b": "B"}}


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


def test_items_view():
    d = Y.YDoc()
    m = d.get_map("test")

    with d.begin_transaction() as txn:
        vals = {"a": 1, "b": 2, "c": 3}
        m.update(txn, vals)
        items = m.items()
        # Ensure that the item view is a multi use iterator
        for _ in range(2):
            expected = vals.copy()
            for (key, val) in items:
                v = expected[key]
                assert val == v
                del expected[key]

        assert len(items) == 3
        assert ("b", 2) in items

        # Ensure that the item view stays up to date with map state
        m.set(txn, "d", 4)
        assert ("d", 4) in items


def test_keys_values():
    d = Y.YDoc()
    m = d.get_map("test")
    expected_keys = list("abc")
    expected_values = list(range(1, 4))
    with d.begin_transaction() as txn:
        m.update(txn, zip(expected_keys, expected_values))

    # Ensure basic iteration works
    for key in m:
        assert key in expected_keys
        assert key in m

    # Ensure keys can be iterated over multiple times
    keys = m.keys()
    for _ in range(2):
        for key in keys:
            assert key in expected_keys
            assert key in keys

    values = m.values()

    for _ in range(2):
        for val in values:
            assert val in expected_values
            assert val in values

    # Ensure keys and values reflect updates to map
    with d.begin_transaction() as txn:
        m.set(txn, "d", 4)

    assert "d" in keys
    assert 4 in values

    # Ensure key view operations
    assert len(keys) == 4
    assert len(values) == 4


def test_observer():
    d1 = Y.YDoc()
    x = d1.get_map("test")
    target = None
    entries = None

    def callback(e: YMapEvent):
        nonlocal target
        nonlocal entries
        target = e.target
        entries = e.keys

    observation_id = x.observe(callback)

    # insert initial data to an empty YMap
    with d1.begin_transaction() as txn:
        x.set(txn, "key1", "value1")
        x.set(txn, "key2", 2)

    assert dict(target) == dict(x)
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

    assert dict(target) == dict(x)
    assert entries == {
        "key1": {"action": "delete", "oldValue": "value1"},
        "key2": {"action": "update", "oldValue": 2, "newValue": "value2"},
    }

    target = None
    entries = None

    # free the observer and make sure that callback is no longer called
    x.unobserve(observation_id)
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
        assert len(e[0].path()) == 1

    sub = container.observe_deep(callback)
    with doc.begin_transaction() as txn:
        container["inner"].set(txn, "addition", 1)

    events = None

    container.unobserve_deep(sub)
    with doc.begin_transaction() as txn:
        container["inner"].set(txn, "don't show up", 1)

    assert events is None


def test_borrow_issue():
    doc = Y.YDoc()
    wrapper = doc.get_array("wrapper")
    inner = Y.YMap({"Foo": "Bar"})

    with doc.begin_transaction() as txn:
        wrapper.append(txn, inner)

def test_integrate_nested_map():
    d1 = Y.YDoc()
    array = d1.get_array("test")
    nested = Y.YMap({"a": Y.YArray(["A"])})

    d1.transact(lambda txn: array.append(txn, nested))
    assert array[0]["a"][0] == "A"
