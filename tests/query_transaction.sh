#!/bin/bash

HOST=$1
TRANSACTION=$2

clickhouse-client --host $HOST --port 9000 --database tracer_api_db --query "SELECT message FROM transactions T, evm_transactions E WHERE transaction_signature IN (SELECT transaction_signature FROM evm_transactions WHERE eth_transaction_signature = '${TRANSACTION}')"
