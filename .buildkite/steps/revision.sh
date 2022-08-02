#!/bin/bash

set ${NEON_EVM_REVISION:=ci-solana-v1.11.3-dumper-plugin}
set ${SOLANA_REVISION:=v1.11.3}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_REVISION}"
echo "Solana: ${SOLANA_REVISION}"