import y_py as Y

def test_before_state():
    doc = Y.YDoc()
    text = doc.get_text("test")
    with doc.begin_transaction() as txn:
        text.extend(txn, "Hello")
        assert txn.before_state == {}
    with doc.begin_transaction() as txn:
        text.extend(txn, " World")
        assert len(txn.before_state) == 1
    