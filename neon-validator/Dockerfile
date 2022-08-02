ARG NEON_EVM_COMMIT
ARG SOLANA_IMAGE
ARG ACCOUNT_DUMPER_IMAGE

FROM ${SOLANA_IMAGE} AS solana

FROM ${ACCOUNT_DUMPER_IMAGE} AS neon-dumper-plugin

FROM neonlabsorg/evm_loader:${NEON_EVM_COMMIT}

WORKDIR /opt

COPY --from=solana \
  /opt/solana/bin/solana \
  /opt/solana/bin/solana-validator \
  /opt/solana/bin/solana-keygen \
  /opt/solana/bin/solana-faucet \
  /opt/solana/bin/solana-genesis \
  /opt/solana/run.sh \
  /opt/solana/fetch-spl.sh \
  /opt/solana/bin/

COPY --from=neon-dumper-plugin \
    /opt/libneon_dumper_plugin.so \
    /opt/
