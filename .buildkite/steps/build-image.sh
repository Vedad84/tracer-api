#!/bin/bash
set -euo pipefail
set ${NEON_EVM_COMMIT:=ci-tracing-api-v0.5.1}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_COMMIT}"

docker build -t neonlabsorg/tracer-db:${BUILDKITE_COMMIT} ./clickhouse
docker build -t neonlabsorg/neon-tracer:${BUILDKITE_COMMIT} --build-arg NEON_REVISION=${NEON_EVM_COMMIT} .
docker build -t neonlabsorg/neon-tracer-test:${BUILDKITE_COMMIT} ./tests
