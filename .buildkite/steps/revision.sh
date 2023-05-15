#!/bin/bash

set ${NEON_REVISION:=3a01fd3cd892263f8f3dbb17951a2adb011bedaa}

echo "Tracer API revision=${BUILDKITE_COMMIT}"
echo "Neon EVM revision=${NEON_REVISION}"
