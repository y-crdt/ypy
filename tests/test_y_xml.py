import unittest

import y_py as Y
from test_helper import exchange_updates


def test_insert():
    d1 = Y.YDoc()
    root = d1.get_xml_element("test")
    with d1.begin_transaction() as txn:
        b = root.push_xml_text(txn)
        a = root.insert_xml_element(txn, 0, "p")
        aa = a.push_xml_text(txn)

        aa.push(txn, "hello")
        b.push(txn, "world")

    s = str(root)
    assert s == "<test><p>hello</p>world</test>"


def test_attributes():
    d1 = Y.YDoc()
    root = d1.get_xml_element("test")
    with d1.begin_transaction() as txn:
        root.set_attribute(txn, "key1", "value1")
        root.set_attribute(txn, "key2", "value2")

        actual = {}
        for key, value in root.attributes():
            actual[key] = value
    assert actual == {"key1": "value1", "key2": "value2"}

    with d1.begin_transaction() as txn:
        root.remove_attribute(txn, "key1")
        actual = {
            "key1": root.get_attribute("key1"),
            "key2": root.get_attribute("key2"),
        }

    assert actual == {"key1": None, "key2": "value2"}


def test_siblings():
    d1 = Y.YDoc()
    root = d1.get_xml_element("test")
    with d1.begin_transaction() as txn:
        b = root.push_xml_text(txn)
        a = root.insert_xml_element(txn, 0, "p")
        aa = a.push_xml_text(txn)

        aa.push(txn, "hello")
        b.push(txn, "world")
        first = a
        assert first.prev_sibling == None

        second = first.next_sibling
        s = str(second)
        assert s == "world"
        assert second.next_sibling == None

    actual = str(second.prev_sibling)
    expected = str(first)
    assert actual == expected


def test_tree_walker():
    d1 = Y.YDoc()
    root = d1.get_xml_element("test")
    with d1.begin_transaction() as txn:
        b = root.push_xml_text(txn)
        a = root.insert_xml_element(txn, 0, "p")
        aa = a.push_xml_text(txn)
        aa.push(txn, "hello")
        b.push(txn, "world")

    actual = [str(child) for child in root.tree_walker()]
    expected = ["<p>hello</p>", "hello", "world"]
    assert actual == expected


def test_xml_text_observer():
    d1 = Y.YDoc()

    x = d1.get_xml_text("test")
    target = None
    attributes = None
    delta = None

    def callback(e):
        nonlocal target
        nonlocal attributes
        nonlocal delta
        target = e.target
        attributes = e.keys
        delta = e.delta

    subscription_id = x.observe(callback)

    # set initial attributes
    with d1.begin_transaction() as txn:
        x.set_attribute(txn, "attr1", "value1")
        x.set_attribute(txn, "attr2", "value2")

    assert str(target) == str(x)
    assert delta == []
    assert attributes == {
        "attr1": {"action": "add", "newValue": "value1"},
        "attr2": {"action": "add", "newValue": "value2"},
    }
    target = None
    attributes = None
    delta = None

    # update attributes
    with d1.begin_transaction() as txn:
        x.set_attribute(txn, "attr1", "value11")
        x.remove_attribute(txn, "attr2")

    assert str(target) == str(x)
    assert delta == []
    assert attributes == {
        "attr1": {"action": "update", "oldValue": "value1", "newValue": "value11"},
        "attr2": {"action": "delete", "oldValue": "value2"},
    }
    target = None
    attributes = None
    delta = None

    # insert initial data to an empty YText
    with d1.begin_transaction() as txn:
        x.insert(txn, 0, "abcd")
    assert str(target), str(x)
    assert delta == [{"insert": "abcd"}]
    assert attributes == {}
    target = None
    attributes = None
    delta = None
    # remove 2 chars from the middle
    with d1.begin_transaction() as txn:
        x.delete(txn, 1, 2)
    assert str(target) == str(x)
    assert delta == [{"retain": 1}, {"delete": 2}]
    assert attributes == {}
    target = None
    attributes = None
    delta = None

    # insert item in the middle
    with d1.begin_transaction() as txn:
        x.insert(txn, 1, "e")
    assert str(target) == str(x)
    assert delta == [{"retain": 1}, {"insert": "e"}]
    assert attributes == {}
    target = None
    attributes = None
    delta = None

    # free the observer and make sure that callback is no longer called
    x.unobserve(subscription_id)
    with d1.begin_transaction() as txn:
        x.insert(txn, 1, "fgh")
    assert target == None
    assert attributes == None
    assert delta == None


def test_xml_element_observer():
    d1 = Y.YDoc()

    x = d1.get_xml_element("test")
    target = None
    attributes = None
    nodes = None

    def callback(e):
        nonlocal target
        nonlocal attributes
        nonlocal nodes
        target = e.target
        attributes = e.keys
        nodes = e.delta

    subscription_id = x.observe(callback)

    # insert initial attributes
    with d1.begin_transaction() as txn:
        x.set_attribute(txn, "attr1", "value1")
        x.set_attribute(txn, "attr2", "value2")

    assert str(target) == str(x)
    assert nodes == []
    assert attributes == {
        "attr1": {"action": "add", "newValue": "value1"},
        "attr2": {"action": "add", "newValue": "value2"},
    }
    target = None
    attributes = None
    nodes = None

    # update attributes
    with d1.begin_transaction() as txn:
        x.set_attribute(txn, "attr1", "value11")
        x.remove_attribute(txn, "attr2")

    assert str(target), str(x)
    assert nodes == []
    assert attributes == {
        "attr1": {"action": "update", "oldValue": "value1", "newValue": "value11"},
        "attr2": {"action": "delete", "oldValue": "value2"},
    }

    target = None
    attributes = None
    nodes = None

    # add children
    with d1.begin_transaction() as txn:
        x.insert_xml_element(txn, 0, "div")
        x.insert_xml_element(txn, 1, "p")

    assert str(target) == str(x)
    assert len(nodes[0]["insert"]) == 2  # [{ insert: [div, p] }
    assert attributes == {}
    target = None
    attributes = None
    nodes = None

    # remove a child
    with d1.begin_transaction() as txn:
        x.delete(txn, 0, 1)
    assert str(target) == str(x)
    assert nodes == [{"delete": 1}]
    assert attributes == {}
    target = None
    attributes = None
    nodes = None

    # insert child again
    with d1.begin_transaction() as txn:
        txt = x.insert_xml_text(txn, len(x))

    assert str(target) == str(x)
    assert nodes[0] == {"retain": 1}
    assert nodes[1]["insert"] != None
    assert attributes == {}
    target = None
    attributes = None
    nodes = None

    # free the observer and make sure that callback is no longer called
    x.unobserve(subscription_id)
    with d1.begin_transaction() as txn:
        x.insert_xml_element(txn, 0, "head")
    assert target == None
    assert nodes == None
    assert attributes == None


def test_deep_observe():
    ydoc = Y.YDoc()
    container = ydoc.get_xml_element("container")
    with ydoc.begin_transaction() as txn:
        text = container.insert_xml_text(txn, 0)

    events = None

    def callback(e: list):
        nonlocal events
        events = e

    sub = container.observe_deep(callback)
    with ydoc.begin_transaction() as txn:
        container.first_child.push(txn, "nested")

    assert events != None


def test_xml_fragment():
    ydoc = Y.YDoc()
    fragment = ydoc.get_xml_fragment("fragment")
    with ydoc.begin_transaction() as txn:
        fragment.insert_xml_element(txn, 0, "a")
        fragment.insert_xml_element(txn, 1, "b")
        fragment.insert_xml_element(txn, 2, "c")

    assert len(fragment) == 3
    assert str(fragment) == "<a></a><b></b><c></c>"
    first_child = fragment.first_child
    with ydoc.begin_transaction() as txn:
        first_child.set_attribute(txn, "key", "value")
        fragment.delete(txn, 1, 1)

    assert str(fragment) == '<a key="value"></a><c></c>'

    assert fragment.get(2) is None

    c_node = fragment.get(1)
    with ydoc.begin_transaction() as txn:
        c_node.insert_xml_element(txn, 0, "d")

    actual = [str(child) for child in fragment.tree_walker()]
    expected = ['<a key="value"></a>', "<c><d></d></c>", "<d></d>"]
    assert actual == expected
