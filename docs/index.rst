Ypy Documentation
================================
Ypy is a high-performance CRDT that allows Python developers to easily synchronize state between processes. It is built on top of Y-CRDT: a powerful distributed data type library written in Rust. With Ypy, developers can make robust, eventually consistent applications that share state between users. All changes are automatically resolved across application instances, so your code can focus on representing state instead of synchronizing it. This shared state can go beyond Python programs, interfacing to web applications backed by Y-Wasm. This allows for seamless communication between frontend user interfaces and Python application logic.

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   installation
   tutorial
   license






Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
