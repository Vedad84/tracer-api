#!/bin/bash

neon-cli --commitment confirmed --url $SOLANA_URL --evm_loader "$EVM_LOADER" neon-elf-params
export $(neon-cli --commitment confirmed --url $SOLANA_URL --evm_loader "$EVM_LOADER" neon-elf-params)

neon-tracer -l $LISTEN_ADDR \
      -h $TRACER_DB_HOST \
      -P $TRACER_DB_PORT \
      -d $TRACER_DB_NAME \
      -u $TRACER_DB_USER \
      -p $TRACER_DB_PASSWORD \
      --evm-loader $EVM_LOADER \
      -w $WEB3_PROXY \
      -i $METRICS_IP \
      -m $METRICS_PORT
