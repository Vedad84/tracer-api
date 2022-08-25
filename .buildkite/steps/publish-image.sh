#!/bin/bash
set -euo pipefail

REVISION=$(git rev-parse HEAD)

docker login -u $DHUBU -p $DHUBP

if [[ ${BUILDKITE_BRANCH} == "master" ]]; then
    TAG=stable
elif [[ ${BUILDKITE_BRANCH} == "develop" ]]; then
    TAG=latest
else
    TAG=${BUILDKITE_BRANCH}
fi

echo "Publishing images with tag: ${TAG}"

docker pull neonlabsorg/neon-tracer:${REVISION}
docker tag neonlabsorg/neon-tracer:${REVISION} neonlabsorg/neon-tracer:${TAG}
docker push neonlabsorg/neon-tracer:${TAG}

docker pull neonlabsorg/neon-tracer-test:${REVISION}
docker tag neonlabsorg/neon-tracer-test:${REVISION} neonlabsorg/neon-tracer-test:${TAG}
docker push neonlabsorg/neon-tracer-test:${TAG}

docker pull neonlabsorg/neon-rpc:${REVISION}
docker tag neonlabsorg/neon-rpc:${REVISION} neonlabsorg/neon-rpc:${TAG}
docker push neonlabsorg/neon-rpc:${TAG}
