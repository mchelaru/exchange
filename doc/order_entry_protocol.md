# General workflow

After the TCP connect, a client is expected to start by sending the login sequeunce.

## Login

The login sequence consists in a login packet (together with an OEP header). If the login is correct then the gateway will echo back the login packet. Otherwise, no answer will be sent. It's up to the client to implement a fallback mechanism for the login failed case.

# Messages

All representations are small endian.
Sequencing is left to the transport protocol. The execution reports contain enough data to match the initial order.

## Header

Each packet need to start with a fixed sized header:

```
| Version (2) | Type (2) | Length (4) |
```

Version - current version is 1

Type -      0 => MsgType::NewOrder,
            1 => MsgType::Modify,
            2 => MsgType::Cancel,
            3 => MsgType::ExecutionReport,
            4 => MsgType::Login,

Length - represents the length of the inner message (without this header)

## New Order

```
| clordid(8) | participant(8) | book_id(8) | quantity(8) | ord_type(2) | side(1) | gateway_id(1) | session_id(4) |
```

## Modify

```
| participant(8) | order_id(8) | book_id(8) | quantity(8) | price(8) | side(1) | gateway_id(1) | session_id(4) |
```

NB The order_id is the id returned in the execution report, and not the client order id.

## Cancel

```
| participant(8) | order_id(8) | book_id(8) | side(1) | gateway_id(1) | session_id(4) |
```

    pub participant: u64,
    pub order_id: u64,
    pub book_id: u64,
    pub side: u8,
    pub gateway_id: u8,
    pub session_id: u32,


## Execution report

    pub participant: u64,
    pub order_id: u64,
    pub submitted_order_id: u64, // client_order_id for new order or exchange_order_id for modifies, cancels
    pub book: u64,
    pub quantity: u64,
    pub price: u64,
    pub flags: u16,
    pub side: u8,
    pub state: u8, // see ExecutionReportType
    pub gateway_id: u8,
    pub session_id: u32,


## Login

```
| participant (8) | session_id (4) | gateway_id (1) | padding (3) | user (64) | password (64) |
```

NB: password needs to be hashed using SHA-512

User field is treated like a C-string, that means that the \0 character means EOS.
