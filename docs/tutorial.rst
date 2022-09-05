Tutorial
========

Each user working with Ypy data can read and update information through a shared document instance. Anything added to the document will be tracked and synchronized across all document instances. These documents can hold common data types, including numbers, booleans, strings, lists, dictionaries, and XML trees. Modifying the document state is done inside a transaction for robustness and thread safety. With these building blocks, you can safely share data between users. Here is a basic hello world example:

.. code-block:: python

    import y_py as Y

    d1 = Y.YDoc()
    # Create a new YText object in the YDoc
    text = d1.get_text('test')
    # Start a transaction in order to update the text
    with d1.begin_transaction() as txn:
        # Add text contents
        text.extend(txn, "hello world!")

    # Create another document
    d2 = Y.YDoc()
    # Share state with the original document
    state_vector = Y.encode_state_vector(d2)
    diff = Y.encode_state_as_update(d1, state_vector)
    Y.apply_update(d2, diff)

    value = str(d2.get_text('test'))

    assert value == "hello world!"