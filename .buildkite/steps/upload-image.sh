#!/bin/bash
set -euo pipefail
source .buildkite/steps/revision.sh

docker images

docker login -u=${DHUBU} -p=${DHUBP}

docker push neonlabsorg/neon-tracer:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-tracer-test:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-rpc:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-dumper-plugin:${BUILDKITE_COMMIT}
docker push neonlabsorg/neon-validator:${BUILDKITE_COMMIT}
