#!/bin/bash

set ${NEON_REVISION:=latest}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"
