from typing import (
    Any,
    Callable,
    Iterator,
    List,
    Iterable,
    Literal,
    Optional,
    Tuple,
    TypedDict,
    Union,
    Dict,
)

SubscriptionId = int

class YDoc:
    """
    A Ypy document type. Documents are most important units of collaborative resources management.
    All shared collections live within a scope of their corresponding documents. All updates are
    generated on per document basis (rather than individual shared type). All operations on shared
    collections happen via YTransaction, which lifetime is also bound to a document.

    Document manages so called root types, which are top-level shared types definitions (as opposed
    to recursively nested types).

    Example::

        from y_py import YDoc

        doc = YDoc()
        with doc.begin_transaction() as txn:
            text = txn.get_text('name')
            text.push(txn, 'hello world')
            output = text.to_string(txn)
            print(output)
    """

    client_id: int
    def __init__(
        self,
        client_id: Optional[int],
        offset_kind: Optional[str],
        skip_gc: Optional[bool],
    ):
        """
        Creates a new Ypy document. If `client_id` parameter was passed it will be used as this
        document globally unique identifier (it's up to caller to ensure that requirement).
        Otherwise it will be assigned a randomly generated number.
        """
    def begin_transaction(self) -> YTransaction:
        """

        Returns:
            A new transaction for this document. Ypy shared data types execute their
            operations in a context of a given transaction. Each document can have only one active
            transaction at the time - subsequent attempts will cause exception to be thrown.

        Transactions started with `doc.begin_transaction` can be released by deleting the transaction object
        method.

        Example::

            from y_py import YDoc
            doc = YDoc()
            text = doc.get_text('name')
            with doc.begin_transaction() as txn:
                text.insert(txn, 0, 'hello world')

        """
    def transact(self, callback: Callable[[YTransaction]]): ...
    def get_map(self, name: str) -> YMap:
        """
        Returns:
            A `YMap` shared data type, that's accessible for subsequent accesses using given `name`.

        If there was no instance with this name before, it will be created and then returned.

        If there was an instance with this name, but it was of different type, it will be projected
        onto `YMap` instance.
        """
    def get_xml_element(self, name: str) -> YXmlElement:
        """
        Returns:
            A `YXmlElement` shared data type, that's accessible for subsequent accesses using given `name`.

        If there was no instance with this name before, it will be created and then returned.

        If there was an instance with this name, but it was of different type, it will be projected
        onto `YXmlElement` instance.
        """
    def get_xml_text(self, name: str) -> YXmlText:
        """
        Returns:
            A `YXmlText` shared data type, that's accessible for subsequent accesses using given `name`.

        If there was no instance with this name before, it will be created and then returned.

        If there was an instance with this name, but it was of different type, it will be projected
        onto `YXmlText` instance.
        """
    def get_array(self, name: str) -> YArray:
        """
        Returns:
            A `YArray` shared data type, that's accessible for subsequent accesses using given `name`.

        If there was no instance with this name before, it will be created and then returned.

        If there was an instance with this name, but it was of different type, it will be projected
        onto `YArray` instance.
        """
    def get_text(self, name: str) -> YText:
        """

        Args:
            name: The identifier for retreiving the text
        Returns:
            A `YText` shared data type, that's accessible for subsequent accesses using given `name`.

        If there was no instance with this name before, it will be created and then returned.
        If there was an instance with this name, but it was of different type, it will be projected
        onto `YText` instance.
        """

StateVector = List[int]

def encode_state_vector(doc: YDoc) -> StateVector:
    """
    Encodes a state vector of a given Ypy document into its binary representation using lib0 v1
    encoding. State vector is a compact representation of updates performed on a given document and
    can be used by `encode_state_as_update` on remote peer to generate a delta update payload to
    synchronize changes between peers.

    Example::

        from y_py import YDoc, encode_state_vector, encode_state_as_update, apply_update from y_py

        # document on machine A
        local_doc = YDoc()
        local_sv = encode_state_vector(local_doc)

        # document on machine B
        remote_doc = YDoc()
        remote_delta = encode_state_as_update(remote_doc, local_sv)

        apply_update(local_doc, remote_delta)

    """

YDocUpdate = List[int]

def encode_state_as_update(doc: YDoc, vector: Optional[StateVector]) -> YDocUpdate:
    """
    Encodes all updates that have happened since a given version `vector` into a compact delta
    representation using lib0 v1 encoding. If `vector` parameter has not been provided, generated
    delta payload will contain all changes of a current Ypy document, working effectively as its
    state snapshot.

    Example::

        from y_py import YDoc, encode_state_vector, encode_state_as_update, apply_update

        # document on machine A
        local_doc = YDoc()
        local_sv = encode_state_vector(local_doc)

        # document on machine B
        remote_doc = YDoc()
        remote_delta = encode_state_as_update(remote_doc, local_sv)

        apply_update(local_doc, remote_delta)
    """

def apply_update(doc: YDoc, diff: YDocUpdate):
    """
    Applies delta update generated by the remote document replica to a current document. This
    method assumes that a payload maintains lib0 v1 encoding format.

    Example::

        from y_py import YDoc, encode_state_vector, encode_state_as_update, apply_update

        # document on machine A
        local_doc = YDoc()
        local_sv = encode_state_vector(local_doc)

        # document on machine B
        remote_doc = YDoc()
        remote_delta = encode_state_as_update(remote_doc, local_sv)

        apply_update(local_doc, remote_delta)
    """

class YTransaction:
    """
    A transaction that serves as a proxy to document block store. Ypy shared data types execute
    their operations in a context of a given transaction. Each document can have only one active
    transaction at the time - subsequent attempts will cause exception to be thrown.

    Transactions started with `doc.begin_transaction` can be released by deleting the transaction object
    method.

    Example::

        from y_py import YDoc
        doc = YDoc()
        text = doc.get_text('name')
        with doc.begin_transaction() as txn:
            text.insert(txn, 0, 'hello world')
    """

    def get_text(self, name: str) -> YText:
        """
        Returns:
            A `YText` shared data type, that's accessible for subsequent accesses using given `name`.

        If there was no instance with this name before, it will be created and then returned.

        If there was an instance with this name, but it was of different type, it will be projected
        onto `YText` instance.
        """
    def get_array(self, name: str) -> YArray:
        """
        Returns:
            A `YArray` shared data type, that's accessible for subsequent accesses using given `name`.

        If there was no instance with this name before, it will be created and then returned.

        If there was an instance with this name, but it was of different type, it will be projected
        onto `YArray` instance.
        """
    def get_map(self, name: str) -> YMap:
        """
        Returns:
            A `YMap` shared data type, that's accessible for subsequent accesses using given `name`.

        If there was no instance with this name before, it will be created and then returned.

        If there was an instance with this name, but it was of different type, it will be projected
        onto `YMap` instance.
        """
    def commit(self):
        """
        Triggers a post-update series of operations without `free`ing the transaction. This includes
        compaction and optimization of internal representation of updates, triggering events etc.
        Ypy transactions are auto-committed when they are `free`d.
        """
    def state_vector_v1(self) -> List[int]:
        """
        Encodes a state vector of a given transaction document into its binary representation using
        lib0 v1 encoding. State vector is a compact representation of updates performed on a given
        document and can be used by `encode_state_as_update` on remote peer to generate a delta
        update payload to synchronize changes between peers.

        Example::

            from y_py import YDoc

            # document on machine A
            local_doc = YDoc()
            local_txn = local_doc.begin_transaction()

            # document on machine B
            remote_doc = YDoc()
            remote_txn = local_doc.begin_transaction()

            try:
                local_sv = local_txn.state_vector_v1()
                remote_delta = remote_txn.diff_v1(local_sv)
                local_txn.applyV1(remote_delta)
            finally:
                del local_txn
                del remote_txn

        """
    def diff_v1(self, vector: Optional[List[int]]) -> List[int]:
        """
        Encodes all updates that have happened since a given version `vector` into a compact delta
        representation using lib0 v1 encoding. If `vector` parameter has not been provided, generated
        delta payload will contain all changes of a current Ypy document, working effectively as
        its state snapshot.

        Example::

            from y_py import YDoc

            # document on machine A
            local_doc = YDoc()
            local_txn = local_doc.begin_transaction()

            # document on machine B
            remote_doc = YDoc()
            remote_txn = local_doc.begin_transaction()

            try:
                local_sv = local_txn.state_vector_v1()
                remote_delta = remote_txn.diff_v1(local_sv)
                local_txn.applyV1(remote_delta)
            finally:
                del local_txn
                del remote_txn
        """
    def apply_v1(self, diff: List[int]):
        """
        Applies delta update generated by the remote document replica to a current transaction's
        document. This method assumes that a payload maintains lib0 v1 encoding format.

        Example::

            from y_py import YDoc

            # document on machine A
            local_doc = YDoc()
            local_txn = local_doc.begin_transaction()

            # document on machine B
            remote_doc = YDoc()
            remote_txn = local_doc.begin_transaction()

            try:
                local_sv = local_txn.state_vector_v1()
                remote_delta = remote_txn.diff_v1(local_sv)
                local_txn.applyV1(remote_delta)
            finally:
                del local_txn
                del remote_txn
        """
    def __enter__() -> YTransaction: ...
    def __exit__() -> bool: ...

class YText:
    """
    A shared data type used for collaborative text editing. It enables multiple users to add and
    remove chunks of text in efficient manner. This type is internally represented as able
    double-linked list of text chunks - an optimization occurs during `YTransaction.commit`, which
    allows to squash multiple consecutively inserted characters together as a single chunk of text
    even between transaction boundaries in order to preserve more efficient memory model.

    `YText` structure internally uses UTF-8 encoding and its length is described in a number of
    bytes rather than individual characters (a single UTF-8 code point can consist of many bytes).

    Like all Yrs shared data types, `YText` is resistant to the problem of interleaving (situation
    when characters inserted one after another may interleave with other peers concurrent inserts
    after merging all updates together). In case of Yrs conflict resolution is solved by using
    unique document id to determine correct and consistent ordering.
    """

    prelim: bool
    def __init__(self, init: Optional[str]):
        """
        Creates a new preliminary instance of a `YText` shared data type, with its state initialized
        to provided parameter.

        Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
        Once a preliminary instance has been inserted this way, it becomes integrated into Ypy
        document store and cannot be nested again: attempt to do so will result in an exception.
        """
    def __str__(self) -> str:
        """
        Returns:
            The underlying shared string stored in this data type.
        """
    def __repr__(self) -> str:
        """
        Returns:
            The string representation wrapped in 'YText()'
        """
    def __len__(self) -> int:
        """
        Returns:
            The length of an underlying string stored in this `YText` instance, understood as a number of UTF-8 encoded bytes.
        """
    def to_json(self) -> str:
        """
        Returns:
            The underlying shared string stored in this data type.
        """
    def insert(self, txn: YTransaction, index: int, chunk: str):
        """
        Inserts a given `chunk` of text into this `YText` instance, starting at a given `index`.
        """
    def push(self, txn: YTransaction, chunk: str):
        """
        Appends a given `chunk` of text at the end of current `YText` instance.
        """
    def delete(self, txn: YTransaction, index: int, length: int):
        """
        Deletes a specified range of of characters, starting at a given `index`.
        Both `index` and `length` are counted in terms of a number of UTF-8 character bytes.
        """
    def observe(self, f: Callable[[YTextEvent]]) -> SubscriptionId:
        """
        Assigns a callback function to listen to YText updates.

        Args:
            f: Callback function that runs when the text object receives an update.
        Returns:
            A reference to the callback subscription.
        """
    def unobserve(self, subscription_id: SubscriptionId):
        """
        Cancels the observer callback associated with the `subscripton_id`.

        Args:
            subscription_id: reference to a subscription provided by the `observe` method.
        """

class YTextEvent:
    target: YText
    delta: List[YTextDelta]
    def path(self) -> List[Union[int, str]]:
        """
        Returns:
            Array of keys and indexes creating a path from root type down to current instance of shared type (accessible via `target` getter).
        """

YTextDelta = Union[YTextChangeInsert, YTextChangeDelete, YTextChangeRetain]

class YTextChangeInsert(TypedDict):
    insert: str
    attributes: Optional[Any]

class YTextChangeDelete(TypedDict):
    delete: int

class YTextChangeRetain(TypedDict):
    retain: int
    attributes: Optional[Any]

class YArray:
    prelim: bool
    def __init__(init: Optional[Iterable[Any]]):
        """
        Creates a new preliminary instance of a `YArray` shared data type, with its state
        initialized to provided parameter.

        Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
        Once a preliminary instance has been inserted this way, it becomes integrated into Ypy
        document store and cannot be nested again: attempt to do so will result in an exception.
        """
    def __len__(self) -> int:
        """
        Returns:
            Number of elements in the `YArray`
        """
    def __str__(self) -> str:
        """
        Returns:
            The string representation of YArray
        """
    def __repr__(self) -> str:
        """
        Returns:
            The string representation of YArray wrapped in `YArray()`
        """
    def to_json(self) -> List[Any]:
        """
        Converts an underlying contents of this `YArray` instance into their JSON representation.
        """
    def insert(self, txn: YTransaction, index: int, items: List[Any]):
        """
        Inserts a given range of `items` into this `YArray` instance, starting at given `index`.
        """
    def push(self, txn: YTransaction, items: List[Any]):
        """
        Appends a range of `items` at the end of this `YArray` instance.
        """
    def delete(self, txn: YTransaction, index: int, length: int):
        """
        Deletes a range of items of given `length` from current `YArray` instance,
        starting from given `index`.
        """
    def __getitem__(self, index: Union[int, slice]) -> Any:
        """
        Returns:
            The element stored under given `index` or a new list of elements from the slice range.
        """
    def __iter__(self) -> Iterator:
        """
        Returns:
            An iterator that can be used to traverse over the values stored withing this instance of `YArray`.

        Example::

            from y_py import YDoc

            # document on machine A
            doc = YDoc()
            array = doc.get_array('name')

            for item in array:
                print(item)
        """
    def observe(self, f: Callable[[YArrayEvent]]) -> SubscriptionId:
        """
        Assigns a callback function to listen to YArray updates.

        Args:
            f: Callback function that runs when the array object receives an update.
        Returns:
            An identifier associated with the callback subscription.
        """
    def unobserve(self, subscription_id: SubscriptionId):
        """
        Cancels the observer callback associated with the `subscripton_id`.

        Args:
            subscription_id: reference to a subscription provided by the `observe` method.
        """

YArrayObserver = Any

class YArrayEvent:
    target: YArray
    delta: List[ArrayDelta]
    def path(self) -> List[Union[int, str]]:
        """
        Returns:
            Array of keys and indexes creating a path from root type down to current instance of shared type (accessible via `target` getter).
        """

ArrayDelta = Union[ArrayChangeInsert, ArrayChangeDelete, ArrayChangeRetain]

class ArrayChangeInsert(TypedDict):
    insert: List[Any]

class ArrayChangeDelete:
    retain: int

class ArrayChangeRetain:
    delete: int

class YMap:
    prelim: bool
    def __init__(dict: dict):
        """
        Creates a new preliminary instance of a `YMap` shared data type, with its state
        initialized to provided parameter.

        Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
        Once a preliminary instance has been inserted this way, it becomes integrated into Ypy
        document store and cannot be nested again: attempt to do so will result in an exception.
        """
    def __len__(self) -> int:
        """
        Returns:
            The number of entries stored within this instance of `YMap`.
        """
    def __str__(self) -> str:
        """
        Returns:
            The string representation of the `YMap`.
        """
    def __repr__(self) -> str:
        """
        Returns:
            The string representation of the `YMap` wrapped in 'YMap()'
        """
    def to_json(self) -> dict:
        """
        Converts contents of this `YMap` instance into a JSON representation.
        """
    def set(self, txn: YTransaction, key: str, value: Any):
        """
        Sets a given `key`-`value` entry within this instance of `YMap`. If another entry was
        already stored under given `key`, it will be overridden with new `value`.
        """
    def delete(self, txn: YTransaction, key: str):
        """
        Removes an entry identified by a given `key` from this instance of `YMap`, if such exists.
        """
    def __getitem__(self, key: str) -> Any | None:
        """
        Returns:
            Value of an entry stored under given `key` within this instance of `YMap`, or `None` if no such entry existed.
        """
    def __iter__(self) -> Iterator[str]:
        """
        Returns:
            An iterator that traverses all keys of the `YMap` in an unspecified order.
        """
    def items(self) -> Iterator[Tuple[str, Any]]:
        """
        Returns:
            An iterator that can be used to traverse over all entries stored within this instance of `YMap`. Order of entry is not specified.

        Example::

            from y_py import YDoc

            # document on machine A
            doc = YDoc()
            map = doc.get_map('name')
            with doc.begin_transaction() as txn:
                map.set(txn, 'key1', 'value1')
                map.set(txn, 'key2', true)
            for (key, value) in map.items()):
                print(key, value)
        """
    def observe(self, f: Callable[[YMapEvent]]) -> SubscriptionId:
        """
        Assigns a callback function to listen to YMap updates.

        Args:
            f: Callback function that runs when the map object receives an update.
        Returns:
            A reference to the callback subscription. Delete this observer in order to erase the associated callback function.
        """
    def unobserve(self, subscription_id: SubscriptionId):
        """
        Cancels the observer callback associated with the `subscripton_id`.

        Args:
            subscription_id: reference to a subscription provided by the `observe` method.
        """

class YMapEvent:
    target: YMap
    delta: List[Dict]
    keys: List[YMapEventKeyChange]
    def path(self) -> List[Union[int, str]]:
        """
        Returns:
            Array of keys and indexes creating a path from root type down to current instance of shared type (accessible via `target` getter).
        """

class YMapEventKeyChange(TypedDict):
    action: Literal["add", "update", "delete"]
    oldValue: Optional[Any]
    newValue: Optional[Any]

YXmlAttributes = Iterator[Tuple[str, str]]

Xml = Union[YXmlElement, YXmlText]
YXmlTreeWalker = Iterator[Xml]
EntryChange = Dict[Literal["action", "newValue", "oldValue"], Any]

class YXmlElementEvent:
    target: YXmlElement
    keys: Dict[str, EntryChange]
    delta: List[Dict]
    def path(self) -> List[Union[int, str]]:
        """
        Returns a current shared type instance, that current event changes refer to.
        """

class YXmlElement:
    """
    XML element data type. It represents an XML node, which can contain key-value attributes
    (interpreted as strings) as well as other nested XML elements or rich text (represented by
    `YXmlText` type).

    In terms of conflict resolution, `YXmlElement` uses following rules:

    - Attribute updates use logical last-write-wins principle, meaning the past updates are
      automatically overridden and discarded by newer ones, while concurrent updates made by
      different peers are resolved into a single value using document id seniority to establish
      an order.
    - Child node insertion uses sequencing rules from other Yrs collections - elements are inserted
      using interleave-resistant algorithm, where order of concurrent inserts at the same index
      is established using peer's document id seniority.
    """

    name: str
    first_child: Optional[Xml]
    next_sibling: Optional[Xml]
    prev_sibling: Optional[Xml]
    parent: Optional[YXmlElement]
    def __len__(self) -> int:
        """
        Returns a number of child XML nodes stored within this `YXMlElement` instance.
        """
    def insert_xml_element(
        self,
        txn: YTransaction,
        index: int,
        name: str,
    ) -> YXmlElement:
        """
        Inserts a new instance of `YXmlElement` as a child of this XML node and returns it.
        """
    def insert_xml_text(self, txn: YTransaction, index: int) -> YXmlText:
        """
        Inserts a new instance of `YXmlText` as a child of this XML node and returns it.
        """
    def delete(self, txn: YTransaction, index: int, length: int):
        """
        Removes a range of children XML nodes from this `YXmlElement` instance,
        starting at given `index`.
        """
    def push_xml_element(self, txn: YTransaction, name: str) -> YXmlElement:
        """
        Appends a new instance of `YXmlElement` as the last child of this XML node and returns it.
        """
    def push_xml_text(self, txn: YTransaction) -> YXmlText:
        """
        Appends a new instance of `YXmlText` as the last child of this XML node and returns it.
        """
    def __str__(self) -> str:
        """
        Returns:
            A string representation of this XML node.
        """
    def __repr__(self) -> str:
        """
        Returns:
            A string representation wrapped in YXmlElement
        """
    def set_attribute(self, txn: YTransaction, name: str, value: str):
        """
        Sets a `name` and `value` as new attribute for this XML node. If an attribute with the same
        `name` already existed on that node, its value with be overridden with a provided one.
        """
    def get_attribute(self, name: str) -> Optional[str]:
        """
        Returns a value of an attribute given its `name`. If no attribute with such name existed,
        `null` will be returned.
        """
    def remove_attribute(self, txn: YTransaction, name: str):
        """
        Removes an attribute from this XML node, given its `name`.
        """
    def attributes(self) -> YXmlAttributes:
        """
        Returns an iterator that enables to traverse over all attributes of this XML node in
        unspecified order.
        """
    def tree_walker(self) -> YXmlTreeWalker:
        """
        Returns an iterator that enables a deep traversal of this XML node - starting from first
        child over this XML node successors using depth-first strategy.
        """
    def observe(self, f: Callable[[YXmlElementEvent]]) -> SubscriptionId:
        """
        Subscribes to all operations happening over this instance of `YXmlElement`. All changes are
        batched and eventually triggered during transaction commit phase.

        Args:
            f: A callback function that receives update events.
        Returns:
            A `SubscriptionId` that can be used to cancel the observer callback.
        """
    def unobserve(self, subscription_id: SubscriptionId):
        """
        Cancels the observer callback associated with the `subscripton_id`.

        Args:
            subscription_id: reference to a subscription provided by the `observe` method.
        """

class YXmlText:
    next_sibling: Optional[Xml]
    prev_sibling: Optional[Xml]
    parent: Optional[YXmlElement]
    def __len__():
        """
        Returns:
            The length of an underlying string stored in this `YXmlText` instance, understood as a number of UTF-8 encoded bytes.
        """
    def insert(self, txn: YTransaction, index: int, chunk: str):
        """
        Inserts a given `chunk` of text into this `YXmlText` instance, starting at a given `index`.
        """
    def push(self, txn: YTransaction, chunk: str):
        """
        Appends a given `chunk` of text at the end of `YXmlText` instance.
        """
    def delete(self, txn: YTransaction, index: int, length: int):
        """
        Deletes a specified range of of characters, starting at a given `index`.
        Both `index` and `length` are counted in terms of a number of UTF-8 character bytes.
        """
    def __str__(self) -> str:
        """
        Returns:
            The underlying string stored in this `YXmlText` instance.
        """
    def __repr__(self) -> str:
        """
        Returns:
            The string representation wrapped in 'YXmlText()'
        """
    def set_attribute(self, txn: YTransaction, name: str, value: str):
        """
        Sets a `name` and `value` as new attribute for this XML node. If an attribute with the same
        `name` already existed on that node, its value with be overridden with a provided one.
        """
    def get_attribute(self, name: str) -> Optional[str]:
        """
        Returns:
            A value of an attribute given its `name`. If no attribute with such name existed,
        `None` will be returned.
        """
    def remove_attribute(self, txn: YTransaction, name: str):
        """
        Removes an attribute from this XML node, given its `name`.
        """
    def attributes(self) -> YXmlAttributes:
        """
        Returns:
            An iterator that enables to traverse over all attributes of this XML node in
        unspecified order.
        """
    def observe(self, f: Callable[[YXmlTextEvent]]) -> SubscriptionId:
        """
        Subscribes to all operations happening over this instance of `YXmlText`. All changes are
        batched and eventually triggered during transaction commit phase.
        Args:
            f: A callback function that receives update events.
        Returns:
            A `SubscriptionId` that can be used to cancel the observer callback.
        """
    def unobserve(self, subscription_id: SubscriptionId):
        """
        Cancels the observer callback associated with the `subscripton_id`.

        Args:
            subscription_id: reference to a subscription provided by the `observe` method.
        """

class YXmlTextEvent:
    target: YXmlText
    keys: List[EntryChange]
    delta: List[YTextDelta]
    def path(self) -> List[Union[int, str]]:
        """
        Returns a current shared type instance, that current event changes refer to.
        """
