#!/bin/bash
set -euo pipefail
source .buildkite/steps/revision.sh

echo -e "\n\n\nPull Docker Images..."
docker-compose -f docker-compose-test.yml pull