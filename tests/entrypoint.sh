#!/bin/bash

sleep 10 # Remove after switching to heatcheck in faucet

python3 -m unittest discover -v -p 'test*.py'
