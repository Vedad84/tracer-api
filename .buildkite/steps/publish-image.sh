#!/bin/bash
set -euo pipefail

REVISION=$(git rev-parse HEAD)

docker images

docker login -u $DHUBU -p $DHUBP

if [[ ${BUILDKITE_BRANCH} == "master" ]]; then
    TAG=stable
elif [[ ${BUILDKITE_BRANCH} == "develop" ]]; then
    TAG=latest
else
    TAG=${BUILDKITE_BRANCH}
fi

docker pull neonlabsorg/neon-tracer:${REVISION}
docker tag neonlabsorg/neon-tracer:${REVISION} neonlabsorg/neon-tracer:${TAG}
docker push neonlabsorg/neon-tracer:${TAG}

docker pull neonlabsorg/neon-tracer-test:${REVISION}
docker tag neonlabsorg/neon-tracer-test:${REVISION} neonlabsorg/neon-tracer-test:${TAG}
docker push neonlabsorg/neon-tracer-test:${TAG}

docker pull neonlabsorg/neon-rpc:${REVISION}
docker tag neonlabsorg/neon-rpc:${REVISION} neonlabsorg/neon-rpc:${TAG}
docker push neonlabsorg/neon-rpc:${TAG}

docker pull neonlabsorg/neon-dumper-plugin:${REVISION}
docker tag neonlabsorg/neon-dumper-plugin:${REVISION} neonlabsorg/neon-dumper-plugin:${TAG}
docker push neonlabsorg/neon-dumper-plugin:${TAG}

docker pull neonlabsorg/neon-validator:${REVISION}
docker tag neonlabsorg/neon-validator:${REVISION} neonlabsorg/neon-dumper-plugin:${TAG}
docker push neonlabsorg/neon-validator:${TAG}
