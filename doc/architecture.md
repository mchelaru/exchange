# Schema

```
+-----------------+
|      HTTP       | +----------------+
|     Trading     | |                |
|     monitor     | |    Clearing    |         TCP
|       and       | |       and      +----------------------+
|   instrument    | |   instruments  |                      |
|   definition    | |                |                      |
|                 | +--------+-------+                      |
+--------+--------+          |                     +--------+-------+       +-------------+
         |                   |                     |                |  Feed |             |
         |TCP             TCP|                     |   Matching     |  UDP  |     Feed    |
         |                   |                     |    engine      +------->   publisher |
         |           +-------+-------+             |                |       |             |
         +-----------+               |             |                |       +------+------+
              +------+   Database    |             +----+---^-------+              |
              |      |               |                  |   |                      |
              |      +--------+------+                  |   |                      |
              |               |                     Feed|   |Orders                |
              |TCP         TCP|                      UDP|   |UDP                   |
              |               |                         |   |                      |
         +----+-----+    +----+----+                    |   |                      |
         |          |    |         <--------------------+   |                      |
         | Gateway  |    | Gateway +------------------------+                      |Feed
         |          |    |         |                        |                      |UDP
         +-+--+-----+    +---+-----+                        |                      |
           |  |              |                              |                      |
           |  |              |                              |                      |
           |  +---------------------------------------------+                      |
           |                 |                                                     |
           |TCP              |TCP                                                  |
           |                 |                                                     |
        +--+-----------------+-------+                                             |
        |                            |                                             |
        |       Participants         <---------------------------------------------+
        |                            |
        +----------------------------+

```
Generated with asciiflow


N.B. As of now, the feed publisher function is implemented in the matching engine component.

