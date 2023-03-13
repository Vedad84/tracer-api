#!/bin/bash

set ${NEON_REVISION:=latest}
set ${REVISION:=$BUILDKITE_COMMIT}

echo "Tracer API revision=${REVISION}"
echo "Neon EVM revision=${NEON_REVISION}"
