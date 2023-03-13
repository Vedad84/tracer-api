#!/bin/bash
set -euo pipefail
source .buildkite/steps/revision.sh

echo -e "\n\n\nBuilding Tracer-API..."
docker build -t neonlabsorg/neon-tracer:${BUILDKITE_COMMIT} --build-arg NEON_REVISION=${NEON_REVISION} .
echo -e "\n\n\nBuilding Tests..."
docker build -t neonlabsorg/neon-tracer-test:${BUILDKITE_COMMIT} ./tests
echo -e "\n\n\nBuilding Neon-RPC..."
docker build -t neonlabsorg/neon-rpc:${BUILDKITE_COMMIT} ./neon-rpc

echo complete!
