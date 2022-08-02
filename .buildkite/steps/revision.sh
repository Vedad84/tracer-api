#!/bin/bash

set ${NEON_EVM_COMMIT:=ci-solana-v1.11.3-dumper-plugin}
set ${SOLANA_IMAGE:=neonlabsorg/solana:v1.11.3-dumper-plugin}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_COMMIT}"
echo "Solana: ${SOLANA_IMAGE}"