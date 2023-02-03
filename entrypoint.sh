#!/bin/bash

if [ -z "$EVM_RUNTIME_DB_CONFIG_TAR" ]; then
   echo "EVM_RUNTIME_DB_CONFIG_TAR not set"
   exit 1
fi

tar -xf $EVM_RUNTIME_DB_CONFIG_TAR -C /opt/

_term() {
  echo "Caught SIGTERM signal!"
  kill -TERM "$child"
}

trap _term SIGTERM

echo "Starting Neon Tracer API. EVM Runtime DB Config: $EVM_RUNTIME_DB_CONFIG_TAR"
./neon-tracer &

child=$!
wait "$child"
sleep 5