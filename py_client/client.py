import time
import socket
import struct
import numpy as np
from sys import stderr

import oep

GATEWAY_ADDRESS = "rpi4.local"
GATEWAY_PORT = 10000

USERNAME = "test"
PASSWORD = "test"

GATEWAY_ID = 1
SESSION_ID = 600
PARTICIPANT = 1050

BOOK_ID = 1000
BAND_LOW = 100
BAND_HIGH = 150
TICK_SIZE = 0.10
SIDE = 0 # buy
QUANTITY = 10

MAX_ORDERS_IN_FLIGHT = 10000

def main() -> None:
    print("Running")

    protocol = oep.Oep(GATEWAY_ID, SESSION_ID, PARTICIPANT)
    client_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    client_socket.connect((GATEWAY_ADDRESS, GATEWAY_PORT))
    client_socket.send(protocol.build_login(USERNAME, PASSWORD))
    logged_in = False

    total_orders_sent = 0
    order_ids = []

    data = bytes()
    # simple state machine that waits for login confirmation
    # and loops sending one order and waiting for its execution report 
    while True:
        socket_data = client_socket.recv(512)

        if len(socket_data) == 0:
            print("Socket disconnected", file=stderr)
            return
        
        data += socket_data
        if len(data) < 8: # header incomplete
            continue
        msg_len = struct.unpack('<I', data[4:8])[0]
        if len(data) < msg_len + 8:
            continue # wait for the rest of the message

        if data[2] == int(oep.MsgType.LOGIN):
            logged_in = True
        elif data[2] == int(oep.MsgType.EXECUTION_REPORT):
            assert msg_len == 57, f"Invalid len: {len(data)}"
            ereport_type = data[59]
            assert ereport_type == 0, f"Invalid report type {ereport_type} {data}" # need OrderState::Inserted
            order_ids.append(struct.unpack('<Q', data[16:24])[0])
        else:
            print(f"Unknown message type: {data[2]}", file=stderr, flush=True)

        if not logged_in:
            print("Could not login", file=stderr, flush=True)
            return
        data = data[8 + msg_len:]

        if total_orders_sent >= MAX_ORDERS_IN_FLIGHT:
            start = time.perf_counter()
            # retire 10% of the standing orders
            ids_to_retire = np.random.randint(0, MAX_ORDERS_IN_FLIGHT - 1, int(MAX_ORDERS_IN_FLIGHT / 10))
            ids_to_remove = []
            for ids in ids_to_retire:
                current_id = order_ids[ids]
                if current_id in ids_to_remove:
                    continue # dups from the rng
                ids_to_remove.append(current_id)
                client_socket.send(protocol.build_cancel(current_id, BOOK_ID, SIDE))
                _ = client_socket.recv(512) # TODO: check the message
                total_orders_sent -= 1
            for ids in ids_to_remove:
                order_ids.remove(ids)
            stop = time.perf_counter()
            print(f"Deleting {len(ids_to_remove)} orders took {stop - start:0.3f} seconds")
        
        price = np.random.randint(BAND_LOW, BAND_HIGH) # TODO: use the tick size
        client_socket.send(protocol.build_new_day_order(BOOK_ID, QUANTITY, price, SIDE))
        total_orders_sent += 1
        if total_orders_sent % 1000 == 0:
            print(f"Sent {total_orders_sent} orders")



if __name__ == "__main__":
    main()