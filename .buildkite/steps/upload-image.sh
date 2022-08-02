#!/bin/bash
set -euo pipefail
set ${NEON_EVM_COMMIT:=develop}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_COMMIT}"

docker images

docker login -u=${DHUBU} -p=${DHUBP}

docker push neonlabsorg/neon-tracer:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-tracer-test:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-rpc:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-dumper-plugin:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-validator:${BUILDKITE_COMMIT}
