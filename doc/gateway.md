# Order entry gateway

The main role of this component is to filter and relay messages between the clients and the matching engine. There may exists multiple gateway components.

## Example configuration file for gateway.ini
```
[gateway]
id=1

[database]
type=pgsql
address=192.168.0.23
port=5432
username=test
password=secret
database=exchange
```
