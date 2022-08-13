ARG NEON_EVM_REVISION
ARG SOLANA_REVISION
ARG ACCOUNT_DUMPER_REVISION

FROM solanalabs/solana:${SOLANA_REVISION} AS solana

FROM neonlabsorg/neon-dumper-plugin:${ACCOUNT_DUMPER_REVISION} AS neon-dumper-plugin

FROM neonlabsorg/evm_loader:${NEON_EVM_REVISION}

WORKDIR /opt

COPY --from=solana \
    /usr/bin/solana \
    /usr/bin/solana-validator \
    /usr/bin/solana-keygen \
    /usr/bin/solana-faucet \
    /usr/bin/solana-genesis \
    /usr/bin/solana-run.sh \
    /usr/bin/fetch-spl.sh \
    /usr/bin/spl* \
    /opt/solana/bin/

COPY --from=solana /usr/bin/spl-token /opt/spl-token

COPY --from=neon-dumper-plugin \
    /opt/libneon_dumper_plugin.so \
    /opt/

COPY ./accountsdb-plugin-config.json /opt/
