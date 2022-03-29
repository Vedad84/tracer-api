#!/bin/bash
set -euo pipefail
set ${NEON_REVISION:=ci-tracing-api-1.8.12}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"

docker-compose -f docker-compose-test.yml build tracer-db neon-tracer neon-tracer-test
