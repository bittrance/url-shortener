# Demonstration URL shortener

A URL shortener is a sevice where you register a long or complicated URL and receive a compact and simple URL which redirects to the original URL. This is a demonstration of a highly scalable URL shortener written in Rust. The goal is to be able to support millions of redirects per second with limited resources.

## Developing

This is how you run url-shortener locally:

```bash
docker run --rm -d -e POSTGRES_PASSWORD=verrahsecret -p 127.0.0.1:5432:5432 postgres
env POSTGRES_USERNAME=postgres POSTGRES_PASSWORD=verrahsecret POSTGRES_DB=postgres cargo run
```

Remember to prepare the database, see below. You can run `psql "host=localhost user=postgres password=verrahsecret" -c '<SQL>` to achieve this.

The Dockerfile contains the full build instructions if you want to build the container locally.

```bash
docker build -t ghcr.io/bittrance/url-shortener .
```

## Testing url-shortener

Register a new token:

```bash
curl -X POST -H 'Content-Type: application/json' --data-binary '{"target": "https://www.google.com"}' http://localhost:8080/admin/tokens
```

If you then verify that the API returns a response with status code 307 for this token.

```bash
curl -v http://localhost:8080/<token>
```

## Using url-shortener

The URL shortener is published to ghcr.io. You can run it as an OCI container. You want to expose its port 8080 in order to reach the forwarder.

```bash
docker run -p 8080:8080 ghcr.io/bittrance/url-shortener
```

url-shortener is configured using env vars:

| Variables | Description |
| --------- | ----------- |
| BIND_ADDRESS | Address to bind to. Defaults to 0.0.0.0. |
| POOL_SIZE | Max database connections. Defaults to 5. |
| POSTGRES_HOST | PostgreSQL hostname. Defaults to 127.0.0.1. |
| POSTGRES_PORT | PostgreSQL port. Defaults to 5432. |
| POSTGRES_DB | PostgreSQL database. Defaults to urlshortener. |
| POSTGRES_USERNAME | PostgreSQL username. Defaults to url-shortener. |
| POSTGRES_PASSWORD | PostgreSQL password. Required parameter. |

## Database

url-shortener does not yet provision its own tables. You need to apply the following DDL:

```sql
CREATE TABLE tokens (
    token CHAR(8) UNIQUE NOT NULL,
    target VARCHAR(1024) NOT NULL,
    PRIMARY KEY(token)
);

CREATE TABLE counts (
    token CHAR(8),
    target VARCHAR(1024),
    timestamp BIGINT,
    count BIGINT
);
```
