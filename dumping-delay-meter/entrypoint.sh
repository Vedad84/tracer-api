#!/bin/bash

npx hardhat compile
npx hardhat run ./scripts/deploy.js

cat .env
export $(cat .env | xargs) && /usr/bin/dumping-delay-meter
