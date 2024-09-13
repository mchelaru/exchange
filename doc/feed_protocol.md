# The market by order (MBO) feed format

Every message is sent in a separate datagram.

## Header

```
| Sequence (8) | Type ID (1) | Value (var) |
```

| Type ID | Type | Message
--- | --- | ---
| 0 | heartbeat | 
| 1 | instrument | Instrument encoded message (see below)
| 2 | market | Encoded New order message as the described in OEP
| 3 | trade | Encoded Trade message as the described in OEP
| 4 | new order | Encoded New order message as the described in OEP
| 5 | modify | Encoded Modify message as the described in OEP
| 6 | cancel | Encoded Cancel message as the described in OEP

N.B. modify means a modification of a standing order that is not impacting its position in the order queue (e.g. a change in the quoted volume). Contrary to that, a price update will trigger two messages: one cancel and one new order.

## The instrument message format

```
| Sequence (8) | 1 (1) | ID (8) | Type (1) | State (1) | Percentage bands (1) | Percentage variation allowed (1) | Name (variable) |
```
