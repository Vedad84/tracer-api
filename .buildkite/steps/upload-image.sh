#!/bin/bash
set -euo pipefail
set ${NEON_EVM_COMMIT:=ci-tracing-api-develop-v1.8.12}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_COMMIT}"

docker images

docker login -u=${DHUBU} -p=${DHUBP}

docker push neonlabsorg/tracer-db:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-tracer:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-tracer-test:${BUILDKITE_COMMIT}

echo step_complete!