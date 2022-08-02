#!/bin/bash
set -euo pipefail
set ${NEON_EVM_COMMIT:=latest}
set ${SOLANA_IMAGE:=neonlabsorg/solana:v1.11.3-dumper-plugin}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_COMMIT}"
echo "Solana: ${SOLANA_IMAGE}"

echo -e "\n\n\nBuilding Tracer-API..."
docker build -t neonlabsorg/neon-tracer:${BUILDKITE_COMMIT} --build-arg NEON_REVISION=${NEON_EVM_COMMIT} .
echo -e "\n\n\nBuilding Tests..."
docker build -t neonlabsorg/neon-tracer-test:${BUILDKITE_COMMIT} ./tests
echo -e "\n\n\nBuilding Neon-RPC..."
docker build -t neonlabsorg/neon-rpc:${BUILDKITE_COMMIT} ./neon-rpc
echo -e "\n\n\nBuilding Neon Dumper Plugin..."
docker build -t neonlabsorg/neon-dumper-plugin:${BUILDKITE_COMMIT} ./neon-dumper-plugin
echo -e "\n\n\nBuilding Neon Validator..."
docker build \
  -t neonlabsorg/neon-validator:${BUILDKITE_COMMIT} \
  --build-arg NEON_EVM_COMMIT=${NEON_EVM_COMMIT} \
  --build-arg SOLANA_IMAGE=${SOLANA_IMAGE} \
  --build-arg ACCOUNT_DUMPER_IMAGE=neonlabsorg/neon-dumper-plugin:${BUILDKITE_COMMIT} \
  ./neon-validator

echo complete!
