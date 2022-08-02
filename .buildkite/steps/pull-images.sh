#!/bin/bash
set -euo pipefail
set ${NEON_EVM_COMMIT:=develop}

echo "Pull Docker Images..."
echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_COMMIT}"

docker-compose -f docker-compose-test.yml pull