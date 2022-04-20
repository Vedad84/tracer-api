#!/bin/bash

curl -X POST http://faucet:3333/request_neon \
   -H 'Content-Type: application/json'  \
   -d '{"wallet":"0xf71c4DACa893E5333982e2956C5ED9B648818376","amount":1000}'

python3 deploy_contracts_make_transactions.py --target=http://neon-rpc:9090 --tracer=http://neon-rpc:9090

pytest tests/test_geth_traces.py --capture=tee-sys --target=http://neon-rpc:9090  --trace_url=http://neon-rpc:9090
pytest tests/test_open_eth_traces.py --capture=tee-sys --target=http://neon-rpc:9090  --trace_url=http://neon-rpc:9090
python3 -m unittest discover -v -p "test_eth_call.py"
python3 -m unittest discover -v -p "test_get_storage_at.py"
python3 -m unittest discover -v -p "test_get_balance.py"
python3 -m unittest discover -v -p "test_eth_get_code.py"
