#!/bin/bash

if [ -z "$EVM_RUNTIME_DB_CONFIG" ]; then
   echo "EVM_RUNTIME_DB_CONFIG not set"
   exit 1
fi

tar -xf $EVM_RUNTIME_DB_CONFIG -C /opt/

./neon-tracer