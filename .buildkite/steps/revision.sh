#!/bin/bash

set ${NEON_EVM_REVISION:=latest}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_REVISION}"