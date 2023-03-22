#!/bin/bash

set ${NEON_REVISION:=72f95822fe1388adf0d8cbd96c0261188c0510dd}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"
