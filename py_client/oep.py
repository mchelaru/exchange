import hashlib
import numpy as np
import struct
from enum import IntEnum


OEP_VERSION = 1

class MsgType(IntEnum):
    NEW_ORDER = 0
    MODIFY = 1
    CANCEL = 2
    EXECUTION_REPORT = 3
    LOGIN = 4

class Oep:
    def __init__(self, gateway_id: int, session_id: int, participant: int) -> None:
        self.gateway_id = gateway_id
        self.session_id = session_id
        self.participant = participant
        self.client_order_id = 100

    def build_header(self, msg_type: MsgType, msg_len: int) -> bytes:
        return struct.pack('<HHI', OEP_VERSION, int(msg_type), msg_len)

    def build_login(self, username: str, password: str) -> bytes:
        h = hashlib.sha512()
        h.update(password.encode('utf-8'))
        hashed_password = h.digest()
        inner = struct.pack('<QII', self.participant, self.session_id, self.gateway_id) + \
            username.ljust(64, '\0').encode('utf-8') + \
            hashed_password
        assert len(inner) == 144
        return self.build_header(MsgType.LOGIN, len(inner)) + inner
            
    
    def build_new_day_order(self, book_id, quantity, price, side) -> bytes:
        self.client_order_id += 1
        inner = struct.pack('<QQQQQHBBI',
            self.client_order_id - 1, self.participant,
            book_id, quantity, price, 0, side,
            self.gateway_id, self.session_id)
        assert len(inner) == 48
        return self.build_header(MsgType.NEW_ORDER, len(inner)) + inner
            
    def build_cancel(self, order_id: int, book_id: int, side: int) -> bytes:
        inner = struct.pack('<QQQBBI',
            self.participant, order_id, book_id, side,
            self.gateway_id, self.session_id)
        assert len(inner) == 30
        return self.build_header(MsgType.CANCEL, len(inner)) + inner
            