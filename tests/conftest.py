import subprocess

import pytest


@pytest.fixture
def yjs_client():
    p = subprocess.Popen(["node", "tests/ypy_yjs/yjs_client.js"])
    yield p
    p.kill()
