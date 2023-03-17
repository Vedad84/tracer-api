#!/bin/bash

set ${NEON_REVISION:=5a4fa77fd32d25022b6fa51a7651e2bced3d09a0}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"
