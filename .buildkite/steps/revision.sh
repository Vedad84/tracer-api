#!/bin/bash

set ${NEON_REVISION:=627fe7c58a88f3f6802a6257d1edee9d06fc58cc}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"
