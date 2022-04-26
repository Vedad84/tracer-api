#!/bin/bash

neon-cli --commitment confirmed --url $SOLANA_URL --evm_loader "$EVM_LOADER" neon-elf-params
export $(neon-cli --commitment confirmed --url $SOLANA_URL --evm_loader "$EVM_LOADER" neon-elf-params)

neon-tracer -l $LISTEN_ADDR \
      -c $TRACER_DB_URL \
      -d $TRACER_DB_NAME \
      -u $TRACER_DB_USER \
      -p $TRACER_DB_PASSWORD \
      --evm-loader $EVM_LOADER