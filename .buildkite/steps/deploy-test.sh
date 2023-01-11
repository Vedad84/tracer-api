#!/bin/bash
set -euo pipefail
source .buildkite/steps/revision.sh

echo -e "\n\n\nPulling images"
docker-compose -f docker-compose-test.yml pull

echo -e "\n\n\nRunning test"
docker-compose -f docker-compose-test.yml up neon-tracer-test
result=$?

if docker logs validator >validator.log 2>&1; then echo "validator logs saved"; fi
if docker logs evm_loader >evm_loader.log 2>&1; then echo "evm_loader logs saved"; fi
if docker logs dbcreation >dbcreation.log 2>&1; then echo "dbcreation logs saved"; fi
if docker logs faucet >faucet.log 2>&1; then echo "faucet logs saved"; fi
if docker logs indexer >indexer.log 2>&1; then echo "indexer logs saved"; fi
if docker logs neon-tracer >neon-tracer.log 2>&1; then echo "neon-tracer logs saved"; fi
if docker logs proxy >proxy.log 2>&1; then echo "proxy logs saved"; fi
if docker logs neon-rpc >neon-rpc.log 2>&1; then echo "neon-rpc logs saved"; fi
if docker logs neon-tracer-test >neon-tracer-test.log 2>&1; then echo "neon-tracer-test logs saved"; fi

# This is kind a hack to stop all evm containers created by tracer
# TODO: still need to implement graceful shutdown
docker-compose -f docker-compose-test.yml stop neon-tracer neon-rpc
docker ps --filter name=neon_tracer_evm_container* --filter status=running -aq | xargs docker kill
docker ps --filter name=neon_tracer_evm_container* -aq | xargs docker rm

docker-compose -f docker-compose-test.yml down

exit $result
