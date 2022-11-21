#!/bin/bash

set ${NEON_EVM_REVISION:=v0.14.0}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_EVM_REVISION}"
