#!/bin/bash

set ${NEON_REVISION:=58b112166d92a02c511a843df0fe6209d03eb024}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"
