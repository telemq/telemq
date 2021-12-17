# syntax=docker/dockerfile:1
FROM rust:1.56 as builder
WORKDIR /usr/src/telemq
COPY . .
RUN cargo build -p telemq --target-dir dist --release

FROM debian:buster-slim
COPY --from=builder /usr/src/telemq/dist/release/telemq /usr/local/bin/telemq
CMD ["telemq"]