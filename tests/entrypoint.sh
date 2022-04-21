#!/bin/bash

python3 -m unittest discover -v -p "test_eth_call.py"
python3 -m unittest discover -v -p "test_get_storage_at.py"
python3 -m unittest discover -v -p "test_get_balance.py"
python3 -m unittest discover -v -p "test_eth_get_code.py"
python3 -m unittest discover -v -p "test_get_transaction_count.py"
