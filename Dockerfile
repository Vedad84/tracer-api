FROM solanalabs/rust:1.69.0 AS builder

ARG NEON_REVISION
ENV NEON_REVISION $NEON_REVISION

COPY . /opt
WORKDIR /opt
RUN cargo build --release
RUN cargo test --release

FROM ubuntu:20.04
RUN apt-get update && apt install -y ca-certificates && update-ca-certificates --fresh
RUN apt-get install -y libssl-dev

WORKDIR /usr/sbin
COPY --from=builder /opt/target/release/neon-tracer .

ENTRYPOINT [ "./neon-tracer" ]