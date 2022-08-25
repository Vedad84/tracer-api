#!/bin/bash

set ${NEON_EVM_REVISION:=latest}
set ${SOLANA_REVISION:=736c4120ecad16dd70c4c8e7453579f4c79b5e25}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_REVISION}"
echo "Solana: ${SOLANA_REVISION}"