import y_py as Y


def test_set():
    d1 = Y.YDoc()
    x = d1.get_map("test")

    value = d1.transact(lambda txn: x.get(txn, "key"))
    assert value == None

    d1.transact(lambda txn: x.set(txn, "key", "value1"))
    value = d1.transact(lambda txn: x.get(txn, "key"))
    assert value == "value1"

    d1.transact(lambda txn: x.set(txn, "key", "value2"))
    value = d1.transact(lambda txn: x.get(txn, "key"))
    assert value == "value2"


def test_set_nested():
    d1 = Y.YDoc()
    x = d1.get_map("test")
    nested = Y.YMap({"a": "A"})

    # check out to_json(), setting a nested map in set(), adding to an integrated value

    d1.transact(lambda txn: x.set(txn, "key", nested))
    d1.transact(lambda txn: nested.set(txn, "b", "B"))

    json = d1.transact(lambda txn: x.to_json(txn))
    assert json == {"key": {"a": "A", "b": "B"}}


def test_delete():
    d1 = Y.YDoc()
    x = d1.get_map("test")

    d1.transact(lambda txn: x.set(txn, "key", "value1"))
    len = d1.transact(lambda txn: x.length(txn))
    value = d1.transact(lambda txn: x.get(txn, "key"))
    assert len == 1
    assert value == "value1"
    d1.transact(lambda txn: x.delete(txn, "key"))
    len = d1.transact(lambda txn: x.length(txn))
    value = d1.transact(lambda txn: x.get(txn, "key"))
    assert len == 0
    assert value == None

    d1.transact(lambda txn: x.set(txn, "key", "value2"))
    len = d1.transact(lambda txn: x.length(txn))
    value = d1.transact(lambda txn: x.get(txn, "key"))
    assert len == 1
    assert value == "value2"


def test_iterator():
    d1 = Y.YDoc()
    x = d1.get_map("test")

    def test(txn):
        x.set(txn, "a", 1)
        x.set(txn, "b", 2)
        x.set(txn, "c", 3)
        expected = {"a": 1, "b": 2, "c": 3}
        for (key, val) in x.entries(txn):
            v = expected[key]
            assert val == v
            del expected[key]

    d1.transact(test)


def test_observer():
    d1 = Y.YDoc()
    x = d1.get_map("test")
    target = None
    entries = None

    def get_value(x):
        with d1.begin_transaction() as txn:
            return x.to_json(txn)

    def callback(e):
        nonlocal target
        nonlocal entries
        target = e.target
        entries = e.keys

    observer = x.observe(callback)  # TODO: Fix typing

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
        x.delete(txn, "key1")
        x.set(txn, "key2", "value2")

    assert get_value(target) == get_value(x)
    assert entries == {
        "key1": {"action": "delete", "oldValue": "value1"},
        "key2": {"action": "update", "oldValue": 2, "newValue": "value2"},
    }

    target = None
    entries = None

    # free the observer and make sure that callback is no longer called
    del observer
    with d1.begin_transaction() as txn:
        x.set(txn, "key1", [6])
    assert target == None
    assert entries == None
