import pytest
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
    

def test_transaction_already_committed():
    doc = Y.YDoc()
    text = doc.get_text("test")
    with doc.begin_transaction() as txn:
        text.extend(txn, "Hello")

    with pytest.raises(AssertionError) as excinfo:
        text.extend(txn, "Bug")
        
    assert str(excinfo.value) == "Transaction already committed!"
    assert str(text) == "Hello"

    txn = doc.begin_transaction()
    text.extend(txn, "Bug")

    assert str(text) == "HelloBug"
    txn.commit()

    with pytest.raises(AssertionError) as excinfo:
        txn.commit()
    assert str(excinfo.value) == "Transaction already committed!"

    # Try smuggling transaction out of callback and reusing it
    smuggle = {}
    doc.transact(lambda txn: smuggle.update({"txn": txn}))
    with pytest.raises(AssertionError) as excinfo:
        text.extend(smuggle["txn"], "Bug")
    assert str(excinfo.value) == "Transaction already committed!"
    assert str(text) == "HelloBug"


def test_document_modification_during_transaction():
    doc = Y.YDoc()
    text = doc.get_text("test")
    with doc.begin_transaction() as txn:
        with pytest.raises(AssertionError) as excinfo:
            text_2 = doc.get_text("test2")
        assert str(excinfo.value) == "Transaction already started!"

    doc.get_text("test2")
