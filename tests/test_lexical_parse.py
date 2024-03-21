import y_py as Y


# this variable is the markdown version of this data, just for reference--it is not used directly in the tests
MARKDOWN_FOR_REFERENCE = """
a *b*

1. c
   1. d

e
""".strip()

# this is the correctly parsed data
LEXICAL_DATA = {
    "__dir": "ltr",
    "children": [
        {
            "__type": "paragraph",
            "__format": 0,
            "__indent": 0,
            "__dir": "ltr",
            "children": [
                {
                    "__type": "text",
                    "__format": 0,
                    "__style": "",
                    "__mode": 0,
                    "__detail": 0,
                    "text": "a ",
                },
                {
                    "__type": "text",
                    "__format": 1,
                    "__style": "",
                    "__mode": 0,
                    "__detail": 0,
                    "text": "b",
                },
            ],
        },
        {
            "__type": "list",
            "__format": 0,
            "__indent": 0,
            "__dir": "ltr",
            "__listType": "number",
            "__tag": "ol",
            "__start": 1,
            "children": [
                {
                    "__type": "listitem",
                    "__format": 0,
                    "__indent": 0,
                    "__dir": "ltr",
                    "__value": 1,
                    "children": [
                        {
                            "__type": "text",
                            "__format": 0,
                            "__style": "",
                            "__mode": 0,
                            "__detail": 0,
                            "text": "c",
                        }
                    ],
                },
                {
                    "__type": "listitem",
                    "__format": 0,
                    "__indent": 0,
                    "__dir": None,
                    "__value": 2,
                    "children": [
                        {
                            "__type": "list",
                            "__format": 0,
                            "__indent": 0,
                            "__dir": "ltr",
                            "__listType": "number",
                            "__tag": "ol",
                            "__start": 1,
                            "children": [
                                {
                                    "__type": "listitem",
                                    "__format": 0,
                                    "__indent": 0,
                                    "__dir": "ltr",
                                    "__value": 1,
                                    "children": [
                                        {
                                            "__type": "text",
                                            "__format": 0,
                                            "__style": "",
                                            "__mode": 0,
                                            "__detail": 0,
                                            "text": "d",
                                        }
                                    ],
                                }
                            ],
                        }
                    ],
                },
            ],
        },
        {
            "__type": "paragraph",
            "__format": 0,
            "__indent": 0,
            "__dir": "ltr",
            "children": [
                {
                    "__type": "text",
                    "__format": 0,
                    "__style": "",
                    "__mode": 0,
                    "__detail": 0,
                    "text": "e",
                }
            ],
        },
    ],
}

# this is the Y update representation of the same data
RAW_LEXICAL_STATE_AS_UPDATE = b'\x01a\x9c\xb5\xe4\xcf\x0e\x00(\x01\x04root\x05__dir\x01w\x03ltr\x07\x01\x04root\x06(\x00\x9c\xb5\xe4\xcf\x0e\x01\x06__type\x01w\tparagraph(\x00\x9c\xb5\xe4\xcf\x0e\x01\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e\x01\x08__indent\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e\x01\x05__dir\x01w\x03ltr\x07\x00\x9c\xb5\xe4\xcf\x0e\x01\x01(\x00\x9c\xb5\xe4\xcf\x0e\x06\x06__type\x01w\x04text(\x00\x9c\xb5\xe4\xcf\x0e\x06\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e\x06\x07__style\x01w\x00(\x00\x9c\xb5\xe4\xcf\x0e\x06\x06__mode\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e\x06\x08__detail\x01}\x00\x84\x9c\xb5\xe4\xcf\x0e\x06\x01a\x87\x9c\xb5\xe4\xcf\x0e\x01\x06(\x00\x9c\xb5\xe4\xcf\x0e\r\x06__type\x01w\x04list(\x00\x9c\xb5\xe4\xcf\x0e\r\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e\r\x08__indent\x01}\x00!\x00\x9c\xb5\xe4\xcf\x0e\r\x05__dir\x01(\x00\x9c\xb5\xe4\xcf\x0e\r\n__listType\x01w\x06number(\x00\x9c\xb5\xe4\xcf\x0e\r\x05__tag\x01w\x02ol(\x00\x9c\xb5\xe4\xcf\x0e\r\x07__start\x01}\x01\x07\x00\x9c\xb5\xe4\xcf\x0e\r\x06(\x00\x9c\xb5\xe4\xcf\x0e\x15\x06__type\x01w\x08listitem(\x00\x9c\xb5\xe4\xcf\x0e\x15\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e\x15\x08__indent\x01}\x00!\x00\x9c\xb5\xe4\xcf\x0e\x15\x05__dir\x01(\x00\x9c\xb5\xe4\xcf\x0e\x15\x07__value\x01}\x01\x01\x00\x9c\xb5\xe4\xcf\x0e\x15\x01\x00\x05\x81\x9c\xb5\xe4\xcf\x0e\x1b\x01\x84\x9c\xb5\xe4\xcf\x0e\x0c\x01 \x87\x9c\xb5\xe4\xcf\x0e"\x01(\x00\x9c\xb5\xe4\xcf\x0e#\x06__type\x01w\x04text(\x00\x9c\xb5\xe4\xcf\x0e#\x08__format\x01}\x01(\x00\x9c\xb5\xe4\xcf\x0e#\x07__style\x01w\x00(\x00\x9c\xb5\xe4\xcf\x0e#\x06__mode\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e#\x08__detail\x01}\x00\x84\x9c\xb5\xe4\xcf\x0e#\x01b\xa1\x9c\xb5\xe4\xcf\x0e\x11\x01\xa1\x9c\xb5\xe4\xcf\x0e\x19\x01\xa8\x9c\xb5\xe4\xcf\x0e*\x01w\x03ltr\xa8\x9c\xb5\xe4\xcf\x0e+\x01w\x03ltr\x87\x9c\xb5\xe4\xcf\x0e!\x01(\x00\x9c\xb5\xe4\xcf\x0e.\x06__type\x01w\x04text(\x00\x9c\xb5\xe4\xcf\x0e.\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e.\x07__style\x01w\x00(\x00\x9c\xb5\xe4\xcf\x0e.\x06__mode\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e.\x08__detail\x01}\x00\x84\x9c\xb5\xe4\xcf\x0e.\x01c\x81\x9c\xb5\xe4\xcf\x0e\x15\x01\x00\x05\x87\x9c\xb5\xe4\xcf\x0e5\x06(\x00\x9c\xb5\xe4\xcf\x0e;\x06__type\x01w\x08listitem(\x00\x9c\xb5\xe4\xcf\x0e;\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0e;\x08__indent\x01}\x00!\x00\x9c\xb5\xe4\xcf\x0e;\x05__dir\x01(\x00\x9c\xb5\xe4\xcf\x0e;\x07__value\x01}\x02\x07\x00\x9c\xb5\xe4\xcf\x0e;\x06(\x00\x9c\xb5\xe4\xcf\x0eA\x06__type\x01w\x04list(\x00\x9c\xb5\xe4\xcf\x0eA\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0eA\x08__indent\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0eA\x05__dir\x01w\x03ltr(\x00\x9c\xb5\xe4\xcf\x0eA\n__listType\x01w\x06number(\x00\x9c\xb5\xe4\xcf\x0eA\x05__tag\x01w\x02ol(\x00\x9c\xb5\xe4\xcf\x0eA\x07__start\x01}\x01\x07\x00\x9c\xb5\xe4\xcf\x0eA\x06(\x00\x9c\xb5\xe4\xcf\x0eI\x06__type\x01w\x08listitem(\x00\x9c\xb5\xe4\xcf\x0eI\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0eI\x08__indent\x01}\x00!\x00\x9c\xb5\xe4\xcf\x0eI\x05__dir\x01(\x00\x9c\xb5\xe4\xcf\x0eI\x07__value\x01}\x01\xa8\x9c\xb5\xe4\xcf\x0eM\x01w\x03ltr\x07\x00\x9c\xb5\xe4\xcf\x0eI\x01(\x00\x9c\xb5\xe4\xcf\x0eP\x06__type\x01w\x04text(\x00\x9c\xb5\xe4\xcf\x0eP\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0eP\x07__style\x01w\x00(\x00\x9c\xb5\xe4\xcf\x0eP\x06__mode\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0eP\x08__detail\x01}\x00\x84\x9c\xb5\xe4\xcf\x0eP\x01d\x81\x9c\xb5\xe4\xcf\x0eI\x01\x00\x05\x81\x9c\xb5\xe4\xcf\x0e;\x01\x00\x05\xa8\x9c\xb5\xe4\xcf\x0e?\x01~\x87\x9c\xb5\xe4\xcf\x0e\r\x06(\x00\x9c\xb5\xe4\xcf\x0ed\x06__type\x01w\tparagraph(\x00\x9c\xb5\xe4\xcf\x0ed\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0ed\x08__indent\x01}\x00!\x00\x9c\xb5\xe4\xcf\x0ed\x05__dir\x01\xa8\x9c\xb5\xe4\xcf\x0eh\x01w\x03ltr\x07\x00\x9c\xb5\xe4\xcf\x0ed\x01(\x00\x9c\xb5\xe4\xcf\x0ej\x06__type\x01w\x04text(\x00\x9c\xb5\xe4\xcf\x0ej\x08__format\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0ej\x07__style\x01w\x00(\x00\x9c\xb5\xe4\xcf\x0ej\x06__mode\x01}\x00(\x00\x9c\xb5\xe4\xcf\x0ej\x08__detail\x01}\x00\x84\x9c\xb5\xe4\xcf\x0ej\x01e\x01\x9c\xb5\xe4\xcf\x0e\t\x11\x01\x19\x01\x1b\x07*\x025\x06?\x01M\x01W\x0ch\x01'


def make_root():
    """Make the doc, apply the update, and return the root element for future work."""
    doc = Y.YDoc()
    Y.apply_update(doc, RAW_LEXICAL_STATE_AS_UPDATE)
    return doc.get_xml_element("root")


def test_root_type():
    """This test doesn't technically need to pass, but it would be nice to have.
    The root element should be parseable as a `YXmlFragment`.
    Right now there is no known way to get a fragment from the Y interface, but this is the type in JS so it makes sense that these should be fragments here.
    """
    doc = Y.YDoc()
    Y.apply_update(doc, RAW_LEXICAL_STATE_AS_UPDATE)
    root = doc.get_xml_fragment("root")
    assert isinstance(root, Y.YXmlFragment)


def test_children_type():
    """This test doesn't technically need to pass, but it would be nice to have.
    The root element is a `YXmlElement`, which is good, but all of the children are `YXmlText`s, which is not good.
    """
    root = make_root()
    first_child = root.first_child
    assert isinstance(first_child, Y.YXmlElement)


# main tests


def parse_node_option_1(elem):
    """Try to parse the node tree using the first_child and next_sibling properties."""
    result = dict(elem.attributes())

    if isinstance(elem, Y.YXmlText):
        result["text"] = str(elem)
        return result

    children = []
    curr_child_elem = elem.first_child
    while curr_child_elem is not None:
        children.append(parse_node_option_1(curr_child_elem))
        curr_child_elem = curr_child_elem.next_sibling
    result["children"] = children
    return result


def parse_node_option_2(elem):
    """Try to parse the node tree using a tree walker."""
    elems_to_dicts = {elem: dict(elem.attributes())}

    for descendant_elem in elem.tree_walker:
        descendant_dict = dict(descendant_elem.attributes())
        elems_to_dicts[descendant_elem] = descendant_dict
        if descendant_elem.parent is None:
            continue

        parent_dict = elems_to_dicts[descendant_elem.parent]
        if "children" not in parent_dict:
            parent_dict["children"] = []
        parent_dict["children"].append(descendant_dict)

    return elems_to_dicts[elem]


def parse_node_option_3(elem):
    """Try to parse the tree using the yet unimplemented to_delta method, which is the only way to get it to work in JS."""
    result = dict(elem.attributes())

    if hasattr(elem, "first_child"):
        children = []
        current_child = elem.first_child
        while current_child:
            children.append(parse_node_option_3(current_child))
            current_child = current_child.next_sibling
        result["children"] = children

    if hasattr(elem, "to_delta"):
        children = []
        for item in elem.to_delta():
            insert = item.insert
            if not insert:
                continue

            if isinstance(insert, str):
                children[-1]["text"] = insert
                continue

            children.append(parse_node_option_3(insert))
        result["children"] = children

    return result


def test_lexical_parse():
    """This is the main test, and the only one that really needs to pass. It tests fully converting the Y format to JSON."""
    root = make_root()

    # try to parse the node uning any of our options
    result_1 = None
    result_2 = None
    result_3 = None
    try:
        result_1 = parse_node_option_1(root)
    except BaseException:
        pass
    try:
        result_2 = parse_node_option_1(root)
    except BaseException:
        pass
    try:
        result_3 = parse_node_option_1(root)
    except BaseException:
        pass

    # we just need one of these ways to work
    assert any(e == LEXICAL_DATA for e in [result_1, result_2, result_3])
