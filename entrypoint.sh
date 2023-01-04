#!/bin/bash

if [ -z "$EVM_RUNTIME_DB_CONFIG" ]; then
   echo "EVM_RUNTIME_DB_CONFIG not set"
   exit 1
fi

tar -xf $EVM_RUNTIME_DB_CONFIG -C /opt/

_term() {
  echo "Caught SIGTERM signal!"
  kill -TERM "$child" 2>/dev/null
}

trap _term SIGTERM

echo "Starting Neon Tracer API. EVM Runtime DB Config: $EVM_RUNTIME_DB_CONFIG"
./neon-tracer &

child=$!
wait "$child"
