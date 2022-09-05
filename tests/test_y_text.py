from test_helper import exchange_updates
import y_py as Y
from y_py import YText, YTextEvent


def test_to_string():
    expected = "Hello World!"
    expected_json = '"Hello World!"'
    d = Y.YDoc()
    prelim = YText(expected)
    integrated = d.get_text("test")
    with d.begin_transaction() as txn:
        integrated.extend(txn, expected)
    for test in [prelim, integrated]:
        assert str(test) == expected
        assert test.to_json() == expected_json
        assert test.__repr__() == f"YText({expected})"


def test_inserts():
    d1 = Y.YDoc()
    x = d1.get_text("test")
    with d1.begin_transaction() as txn:
        x.extend(txn, "hello ")
        x.extend(txn, "world!")
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

    d1.transact(lambda txn: x.extend(txn, "hello world!"))

    assert len(x) == 12
    with d1.begin_transaction() as txn:
        x.delete_range(txn, 5, 5)
        x.delete(txn, 5)
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

    def callback(e: YTextEvent):
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
        x.delete_range(txn, 1, 2)

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


def test_delta_embed_attributes():

    d1 = Y.YDoc()
    text = d1.get_text("test")

    delta = None

    def callback(e):
        nonlocal delta
        delta = e.delta

    sub = text.observe(callback)

    with d1.begin_transaction() as txn:
        text.insert(txn, 0, "ab", {"bold": True})
        text.insert_embed(txn, 1, {"image": "imageSrc.png"}, {"width": 100})

    expected = [
        {"insert": "a", "attributes": {"bold": True}},
        {"insert": {"image": "imageSrc.png"}, "attributes": {"width": 100}},
        {"insert": "b", "attributes": {"bold": True}},
    ]
    assert delta == expected

    text.unobserve(sub)


def test_formatting():
    d1 = Y.YDoc()
    text = d1.get_text("test")

    delta = None
    target = None

    def callback(e):
        nonlocal delta
        nonlocal target
        delta = e.delta
        target = e.target

    sub = text.observe(callback)

    with d1.begin_transaction() as txn:
        text.insert(txn, 0, "stylish")
        text.format(txn, 0, 4, {"bold": True})

    assert delta == [
        {"insert": "styl", "attributes": {"bold": True}},
        {"insert": "ish"},
    ]

    with d1.begin_transaction() as txn:
        text.format(txn, 4, 7, {"bold": True})

    assert delta == [{"retain": 4}, {"retain": 3, "attributes": {"bold": True}}]

    text.unobserve(sub)


def test_deep_observe():
    d = Y.YDoc()
    text = d.get_text("text")
    with d.begin_transaction() as txn:
        text.extend(txn, "Hello")
    events = None

    def callback(e):
        nonlocal events
        events = e

    sub = text.observe_deep(callback)

    with d.begin_transaction() as txn:
        # Currently, Yrs does not support deep observe on embedded values.
        # Deep observe will pick up the same events as shallow observe.
        text.extend(txn, " World")

    assert events is not None and len(events) == 1

    # verify that the subscription drops
    events = None
    text.unobserve(sub)
    with d.begin_transaction() as txn:
        text.extend(txn, " should not trigger")

    assert events is None
