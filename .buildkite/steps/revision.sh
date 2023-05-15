#!/bin/bash

set ${NEON_REVISION:=a3771afc8bf97bd08d9c0bef4fbf6feeb37b57f5}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"
