#!/bin/bash
set -euo pipefail
set ${NEON_EVM_COMMIT:=develop}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_COMMIT}"

docker build -t neonlabsorg/tracer-db:${BUILDKITE_COMMIT} ./clickhouse
docker build -t neonlabsorg/neon-tracer:${BUILDKITE_COMMIT} --build-arg NEON_REVISION=${NEON_EVM_COMMIT} .
docker build -t neonlabsorg/neon-tracer-test:${BUILDKITE_COMMIT} ./tests
docker build -t neonlabsorg/neon-rpc:${BUILDKITE_COMMIT} ./nginx
echo complete!
