# Matching engine

This is a matching engine that I am using for internal testing, performance measuring and better internals understanding.

This source code comes under 2-clause BSD license. Absolutely no warranty, nor liabilities are given. This software is provided as-is and in case you want to use it, then use it at your own risk. Read more about the licence in the doc/license.md file.

## General notes about architecture

The matching engine is accepting orders and tries to match and post them. It supports running multiple segments at once - e.g. share, options, warrants etc.

Orders are consumed from a multicast socket on a configurable group. The order inserts or deletes and the trades are also broadcast on a configurable multicast group. Ideally, the engine will not be accessed directly by third parties but this interaction should be managed by gateways and feed disseminators.

I am trying to make this project as modular and plugin as possible but be aware that this is not the main goal.

## Instruments

Each instrument contains an ID, a type, a trading state, price bands where orders are accepted, and variation that trigger the instrument going into auction. In general, all the givens are coming from the clearing.

One note on the ID is that its scoped to the matching engine, not to the instrument type. So no two instruments can share the same ID, regardless of their type.

## Matching

This design is implementing a <I>price-time</I> wise matching.

## Sending messages to the matching engine

### Protocol

```
-------------------------------------------------------
| Msg Type (1) | Padding (3) | Original message (var) |
-------------------------------------------------------
```

The original message is the message that was sent by the participant to the gateway. It might be a new order, a modify or a cancel.

| Msg Type | Meaning |
| --- | --- |
| 0 | New Order |
| 1 | Modify |
| 2 | Cancel |
| 6 | Session notification (see below) |

### Session notification

This message is sent in order to notify the matching engine about a certain issue on the session - usually meaning that the client disconnected. As a result, the matching engine kills all the orders of that certain participant/session pair. It has the following format:

```
| Msg Type (1) | Padding (3) | Participant (8) | Session (4) | Gateway (1) |
```

Msg Type = Fixed value, 6
