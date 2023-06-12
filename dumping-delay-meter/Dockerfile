FROM rust:1.68 as builder

COPY . /opt
WORKDIR /opt

RUN cargo build --release

FROM node:16

COPY . /opt

WORKDIR /opt

RUN npm install

COPY --from=builder /opt/target/release/dumping-delay-meter /usr/bin/

ENTRYPOINT /opt/entrypoint.sh
