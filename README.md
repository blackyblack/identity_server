Identity_server
=====

Network of trust implementation as web service.

Build
-----

$ cargo build

Run
---

To start the server use `cargo run`. The service reads `HOST` and `PORT`
environment variables. If they are unset, it defaults to `localhost:8080`.

$ HOST=127.0.0.1 PORT=8080 cargo run

