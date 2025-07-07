Identity_server
=====

Network of trust implementation as web service.

Build
-----

```sh
cargo build
```

Run
---

To start the server use `cargo run`. The service reads `HOST` and `PORT`
environment variables. If they are unset, it defaults to `localhost:8080`.

```sh
HOST=127.0.0.1 PORT=8080 cargo run
```

Database setup
--------------

Create a MySQL database and user:

```
CREATE DATABASE identity;
CREATE USER 'identity'@'localhost' IDENTIFIED BY 'password';
GRANT ALL PRIVILEGES ON identity.* TO 'identity'@'localhost';
```

Environment configuration
-------------------------

The application reads database credentials from environment variables:

- `MYSQL_HOST` (default `localhost`)
- `MYSQL_PORT` (default `3306`)
- `MYSQL_USER` (default `root`)
- `MYSQL_PASSWORD`
- `MYSQL_DATABASE` (default `identity`)
- `SERVER_PRIVATE_KEY` hex-encoded private key for server identity

You can place them in a `.env` file or export them before running the server:

```sh
MYSQL_USER=identity
MYSQL_PASSWORD=password
cargo run
```

