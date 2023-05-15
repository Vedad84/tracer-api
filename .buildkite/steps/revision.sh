#!/bin/bash

set ${NEON_REVISION:=7b7b48e6b5d80eb90f31b8bda1d97f287a2ddefc}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"
