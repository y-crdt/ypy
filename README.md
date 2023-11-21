[![PyPI version](https://badge.fury.io/py/y-py.svg)](https://badge.fury.io/py/y-py)

# Ypy

Ypy is a Python binding for Y-CRDT. It provides distributed data types that enable real-time collaboration between devices. Ypy can sync data with any other platform that has a Y-CRDT binding, allowing for seamless cross-domain communication. The library is a thin wrapper around Yrs, taking advantage of the safety and performance of Rust.

> [We are looking for a maintainer 👀](https://github.com/y-crdt/ypy/issues/148)

## Installation

```
pip install y-py
```

## Getting Started

Ypy provides many of the same shared data types as [Yjs](https://docs.yjs.dev/). All objects are shared within a `YDoc` and get modified within a transaction block.

```python
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
```

## Development Setup

0. Install [Rust](https://www.rust-lang.org/tools/install) and [Python](https://www.python.org/downloads/)
1. Install `maturin` in order to build Ypy: `pip install maturin`
2. Create a development build of the library: `maturin develop`

## Tests

All tests are located in `/tests`. To run the tests, install `pytest` and run the command line tool from the project root:

```
pip install pytest
pytest
```

## Using Hatch

If you are using `hatch`, there is a `test` environment matrix defined in `pyproject.toml` that will run commands in virtual environments for `py37` through `py312`.

```
hatch run test:maturin develop
hatch run test:pytest
```

## Build Ypy 

Build the library as a wheel and store them in `target/wheels`:

```
maturin build
```

## Ypy in WASM (Pyodide)

As a Rust-based library, Ypy cannot build "pure Python" wheels. CI processes build and upload a number of wheels to PyPI, but PyPI does not support hosting `emscripten` / `wasm32` wheels necessary to import in Pyodide (see https://github.com/pypi/warehouse/issues/10416 for more info and updates). For now, Ypy will build `emscripten` wheels and attach the binaries as assets in the appropriate [Releases](https://github.com/y-crdt/ypy/releases) entry. Unfortunately, trying to install directly from the Github download link will result in a CORS error, so you'll need to use a proxy to pull in the binary and write / install from emscripten file system or host the binary somewhere that is CORS accessible for your application.

You can try out Ypy in Pyodide using the [terminal emulator at pyodide.org](https://pyodide.org/en/stable/console.html):

```
Welcome to the Pyodide terminal emulator 🐍
Python 3.10.2 (main, Sep 15 2022 23:28:12) on WebAssembly/Emscripten
Type "help", "copyright", "credits" or "license" for more information.
>>> wheel_url = 'https://github.com/y-crdt/ypy/releases/download/v0.5.5/y_py-0.5.5-cp310-cp310-emscripten_3_1_14_wasm32.whl'
>>> wheel_name = wheel_url.split('/')[-1]
>>> wheel_name
'y_py-0.5.5-cp310-cp310-emscripten_3_1_14_wasm32.whl'
>>> 
>>> proxy_url = f'https://api.allorigins.win/raw?url={wheel_url}'
>>> proxy_url
'https://api.allorigins.win/raw?url=https://github.com/y-crdt/ypy/releases/download/v0.5.5/y_py-0.5.5-cp310-cp310-emscripten_3_1_14_wasm32.whl'
>>> 
>>> import pyodide
>>> resp = await pyodide.http.pyfetch(proxy_url)
>>> resp.status
200
>>> 
>>> content = await resp.bytes()
>>> len(content)
360133
>>> content[:50]
b'PK\x03\x04\x14\x00\x00\x00\x08\x00\xae\xb2}U\x92l\xa7E\xe6\x04\x00\x00u\t\x00\x00\x1d\x00\x00\x00y_py-0.5.5.dist-info'
>>>
>>> with open(wheel_name, 'wb') as f:
...   f.write(content)
... 
360133
>>> 
>>> import micropip
>>> await micropip.install(f'emfs:./{wheel_name}')
>>> 
>>> import y_py as Y
>>> Y
<module 'y_py' from '/lib/python3.10/site-packages/y_py/__init__.py'>
>>> 
>>> d1 = Y.YDoc()
>>> text = d1.get_text('test')
>>> with d1.begin_transaction() as txn:
    text.extend(txn, "hello world!")
... 
>>> d2 = Y.YDoc()
>>> state_vector = Y.encode_state_vector(d2)
>>> diff = Y.encode_state_as_update(d1, state_vector)
>>> Y.apply_update(d2, diff)
>>> d2.get_text('test')
YText(hello world!)
```
