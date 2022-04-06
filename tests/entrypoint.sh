#!/bin/bash

curl -X POST http://faucet:3333/request_neon \
   -H 'Content-Type: application/json'  \
   -d '{"wallet":"0xf71c4DACa893E5333982e2956C5ED9B648818376","amount":1000}'

python3 deploy_contracts_make_transactions.py --target=http://proxy:9090/solana --tracer=http://neon-tracer:8250
#pytest tests/test_geth_traces.py --target=http://neon-tracer:8250
pytest tests/test_open_eth_traces.py --target=http://neon-tracer:8250
 
