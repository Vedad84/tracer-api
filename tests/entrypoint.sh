#!/bin/bash

echo "airdrop 1000"
curl -X POST 'https://api.neonfaucet.org/request_neon' \
     -H 'Content-Type: application/json'  \
     -d '{"wallet":"0xf71c4DACa893E5333982e2956C5ED9B648818376","amount":1000}'

curl --location --request POST 'https://ch-graph.neontest.xyz' \
  --header 'Content-Type: application/json' \
  --data-raw '{ "jsonrpc":"2.0", "method":"eth_getBalance", "params":[ "0xf71c4DACa893E5333982e2956C5ED9B648818376",  "latest" ], "id":1 }'


python3 -m unittest discover -v -p "test_eth_call.py"
python3 -m unittest discover -v -p "test_eth_get_code.py"
python3 -m unittest discover -v -p "test_get_balance.py"
python3 -m unittest discover -v -p "test_get_storage_at.py"
python3 -m unittest discover -v -p "test_get_transaction_count.py"


#python3 deploy_contracts_make_transactions.py --target=https://ch-graph.neontest.xyz --tracer=https://ch-graph.neontest.xyz
#pytest tests/test_geth_traces.py --capture=tee-sys --target=http://neon-rpc:9090  --trace_url=http://neon-rpc:9090

