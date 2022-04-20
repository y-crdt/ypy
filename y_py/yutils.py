from enum import IntEnum
from typing import List


class YMessageType(IntEnum):
    SYNC = 0
    SYNC_STEP1 = 0
    SYNC_STEP2 = 1
    SYNC_UPDATE = 2

def write_var_uint(num):
    res = []
    while num > 127:
        res += [128 | (127 & num)]
        num >>= 7
    res += [num]
    return res

def create_message(data: List[int], msg_type: int) -> bytes:
    return bytes([YMessageType.SYNC, msg_type] + write_var_uint(len(data)) + data)

def create_sync_step1_message(data: List[int]) -> bytes:
    return create_message(data, YMessageType.SYNC_STEP1)

def create_sync_step2_message(data: List[int]) -> bytes:
    return create_message(data, YMessageType.SYNC_STEP2)

def create_update_message(data: List[int]) -> bytes:
    return create_message(data, YMessageType.SYNC_UPDATE)

def get_message(message):
    i = 0
    while True:
        byte = message[i]
        i += 1
        if byte < 128:
            break
    msg = message[i:]
    return msg
