# Loki Redis Pusher

Push log to loki through redis list queue.

## Config

### config.yaml

```yaml
redis:
  host: 127.0.0.1:6379
  # username: Auth username
  # password: Auth password
  # db: 1
  key: loki_push_queue

loki:
  url: http://127.0.0.1:3100/loki/api/v1/push
```

### Env vars
Name | Optional | Default
-- | -- | --
REDIS_HOST | No
REDIS_USERNAME | Yes
REDIS_PASSWORD | Yes
REDIS_DB | Yes | 0
REDIS_KEY | Yes | loki_push_queue
LOKI_URL | Yes | http://127.0.0.1:3100/loki/api/v1/push

## Docker

```shell
docker run -e "REDIS_HOST=192.168.1.100" -e "REDIS_DB=1" --name loki-redis-pusher -d pader/loki-redis-pusher:1.0.0
```