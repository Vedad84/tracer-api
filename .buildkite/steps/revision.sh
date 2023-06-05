#!/bin/bash

set ${NEON_REVISION:=f9f972fd6afcc263df0da49de77ac4c6830df755}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"
