# Demonstration URL shortener

A URL shortener is a sevice where you register a long or complicated URL and receive a compact and simple URL which redirects to the original URL. This is a demonstration of a highly scalable URL shortener written in Rust. The goal is to be able to support millions of redirects per second with limited resources.

## Running

The URL shortener is published to ghcr.io. You can run it as an OCI container. You want to expose its port 8080 in order to reach the forwarder.

```
docker run -p 8080:8080 ghcr.io/bittrance/url-shortener
```

## Building

The Dockerfile contains the full build instructions:
```
docker build -t ghcr.io/bittrance/url-shortener .
```

## Database

```sql
CREATE TABLE tokens (
    token CHAR(8) UNIQUE NOT NULL,
    target VARCHAR(1024) NOT NULL,
    PRIMARY KEY(token)
);
```