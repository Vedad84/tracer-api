#!/bin/bash
set -euo pipefail

docker images

docker login -u=${DHUBU} -p=${DHUBP}

docker-compose -f docker-compose-test.yml push tracer-db neon-tracer neon-tracer-test
