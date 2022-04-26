FROM solanalabs/rust:1.59.0 AS builder

ARG NEON_REVISION
ENV NEON_REVISION $NEON_REVISION

COPY . /opt
WORKDIR /opt
RUN cargo build --release

FROM neonlabsorg/evm_loader:latest as evm

FROM ubuntu:20.04
RUN apt-get update && apt-get install -y libssl-dev

WORKDIR /usr/sbin
COPY --from=builder /opt/target/release/neon-tracer .
COPY --from=evm /opt/neon-cli .
COPY --from=evm /opt/solana/bin/solana .
COPY id.json /root/.config/solana/

COPY ./start_tracer.sh .
CMD ["./start_tracer.sh"]
