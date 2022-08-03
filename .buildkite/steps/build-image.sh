#!/bin/bash
set -euo pipefail
source .buildkite/steps/revision.sh

echo -e "\n\n\nBuilding Tracer-API..."
#docker build -t neonlabsorg/neon-tracer:${BUILDKITE_COMMIT} --build-arg NEON_EVM_REVISION=${NEON_EVM_REVISION} .
echo -e "\n\n\nBuilding Tests..."
docker build -t neonlabsorg/neon-tracer-test:${BUILDKITE_COMMIT} ./tests
echo -e "\n\n\nBuilding Neon-RPC..."
docker build -t neonlabsorg/neon-rpc:${BUILDKITE_COMMIT} ./neon-rpc
echo -e "\n\n\nBuilding Neon Dumper Plugin..."
docker build -t neonlabsorg/neon-dumper-plugin:${BUILDKITE_COMMIT} ./neon-dumper-plugin
echo -r "\n\n\nBuilding Accounts DB..."
docker build -t neonlabsorg/neon-accountsdb:${BUILDKITE_COMMIT} ./neon-dumper-plugin/db
echo -e "\n\n\nBuilding Neon Validator..."
docker build \
  -t neonlabsorg/neon-validator:${BUILDKITE_COMMIT} \
  --build-arg NEON_EVM_REVISION=${NEON_EVM_REVISION} \
  --build-arg SOLANA_REVISION=${SOLANA_REVISION} \
  --build-arg ACCOUNT_DUMPER_REVISION=${BUILDKITE_COMMIT} \
  ./neon-validator

echo complete!
