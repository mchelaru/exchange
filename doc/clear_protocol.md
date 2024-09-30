# The Clear Protocol

This is the protocol that it's being used to communicate between the clearing and the matching engine.

## Specification

The protocol can be transported over TCP or UDP, depending on the local infrastructure. The current document assumes that TCP is used.

All the multi-bytes fields are little-endian.
At this moment the request/responses are not sequenced since there isn't any real need at the moment. This may be subject to change in the future.

### Heartbeats

One peer should disconnect and reconnect if it doesn't receive a heartbeat for 3 seconds.

### Header

Each packet starts with a technical header that specifies the version. Each protocol packet can be split over multiple network packets, as required by the network protocols and infrastructure. The receiver may want to wait for all the entries before starting processing a packet. Format:

```
-------------------------------------
| "CP"(2) | Version(1) | Entries(1) |
-------------------------------------
```

The current protocol version is 1. The maximum packet size should not be more than 10k bytes.

### Data entries

The Clear Protocol is a TLV formatted protocol. The format of each entry is described below. The "Length" field describes the length of the value.

```
---------------------------------------
| Type(2) | Length(2) | Value(Length) |
---------------------------------------
```

### Types

Type | Description | Default length
---|---|---
0 | Heartbeat | 0
1 | Instrument update | 12 + instrument name len (see below)
2 | Instrument request | 8 (instrument ID)
3 | All instruments request | 0

### Instrument update message

ID(8) | Type(1) | State(1) | Percentage bands(1) | Percentage variation(1) | Name(var)
---|---|---|---|---
The instrument ID | Instrument types (see below) | Instrument state (see below) | Percentage bands where orders are allowed to enter and sit vs the current spot | Maximum variation before automatically switching the instrument state into auction | Name of the instrument

Instrument type | Description
---|---
0 | Share
1 | Option call
2 | Option put
3 | Future
4 | Warrant

Instrument state | Description
---|---
0 | Trading
1 | Closed
2 | Auction