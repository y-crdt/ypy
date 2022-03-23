from collections import namedtuple
import json
from typing import TypedDict


class Config(TypedDict):
    host: str
    port: int


def read_config() -> Config:
    """
    Reads config JSON file
    """
    with open("config.json", "r") as f:
        return json.load(f)
