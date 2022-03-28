[![Python 3.9](https://img.shields.io/badge/python-3.9-blue.svg)](https://www.python.org/downloads/release/python-390/)
# EthereumTrace-test
### Usage:

Automation test-framework to execute functional test scenarios against Ethereium EVM tracer or Neon EVM tracer.
Supported trace out formats from existing Ethereum validators:

1. [gETH](https://geth.ethereum.org)
   - [EVM Tracing](https://geth.ethereum.org/docs/dapp/tracing)
2. [OpenEthereum](https://openethereum.github.io)
   - [trace](https://openethereum.github.io/JSONRPC-trace-module.html)

### Quick Start
1. Clone, or download repo
2. Run the **Install Requirements** below.
3. Activate virtual environment with preinstalled dependencies (check **Install Requirements** )
4. Prepare environment: 
   1. NEON EVN with trace or gETH node in dev mode or OpenEthereum node in dev mode (check **Ethereum node in dev mode**)
   2. Deploy test contracts (```*.sol```) (check **Deploy Contracts**)
5. add '.env' file with account private key (PRIVATE_KEY_1={your private key})
6.  **Execute tests**

### Install Requirements
In CMD:
``` 
virtualenv venv

source venv/bin/activate

pip3 install -r requirements.txt
```

### Ethereum node in dev mode

for OpenEthereum [instruction](https://openethereum.github.io/Private-development-chain)
In CMD:
``` 
 openethereum --config dev-insecure --tracing on --gas-price-percentile 0
```
for gETH(you can use accounts from metamask if you want) [instruction](https://geth.ethereum.org/docs/getting-started/dev-mode)
``` 
 geth --dev --http --http.corsdomain="https://remix.ethereum.org" --http.api web3,eth,debug,personal,net --vmdebug --datadir tmp_geth
```

###  Deploy Contracts

Test contracts will be uploaded with transactions to nodes(private ethereum network) and execute 
function from this contract with another transactions.



1. Launch python script in repo ```deploy_contracts_make_transactions.py```
   - by default, it will make RPC connection by address ```http://localhost:8545``` , 
     to change it add argument ```--target {your host}```
     you should see that if everything is ok
   
```
python3 deploy_contracts_make_transactions.py --target=http://localhost:8545

2022-03-02 17:35:06 [common.web3_connection] INFO: Web3 connecting...
2022-03-02 17:35:06 [common.web3_connection] INFO: Web3 connected RPC connection http://localhost:8545 True
2022-03-02 17:35:06 [common.compile] INFO: Installing solidity compiler with solcx --- version 0.4.18
2022-03-02 17:35:06 [common.compile] INFO: Reading file with solidity code --- resources/contracts/Contract_1.sol 
2022-03-02 17:35:06 [common.compile] INFO: Compiling Solidity code
2022-03-02 17:35:06 [common.compile] INFO: Success
2022-03-02 17:35:06 [common.compile] INFO: Installing solidity compiler with solcx --- version 0.4.18
2022-03-02 17:35:06 [common.compile] INFO: Reading file with solidity code --- resources/contracts/Contract_2.sol 
2022-03-02 17:35:06 [common.compile] INFO: Compiling Solidity code
2022-03-02 17:35:06 [common.compile] INFO: Success
2022-03-02 17:35:06 [__main__] INFO: Waiting for transaction to finish...
2022-03-02 17:35:06 [__main__] INFO: Done! Contract deployed to 0x0909518d7619500BA441c2a91e1b1d7025122924
2022-03-02 17:35:06 [__main__] INFO: Contract deployment 'transactionHash': 0x9449eb97c5c033c3134f47cb08ccef2cc788d73a98427b56abd2366fcc25d47b
2022-03-02 17:35:06 [__main__] INFO: Waiting for transaction to finish...
2022-03-02 17:35:06 [__main__] INFO: Done! Contract deployed to 0x54B084DCd2ab3FE5d8F6ddF44C99e8303340DF2a
2022-03-02 17:35:06 [__main__] INFO: Contract deployment 'transactionHash': 0x393cbdb1f09a5a408e4924244b8e81cc375ddd617721ffb0a445dfdacdf647c4
2022-03-02 17:35:06 [__main__] INFO: Set address of first contract with ExistingWithoutABI_func : 0xb6f79a8f157b7acfb70b116c564f303da651f165dd818a9915aca644b887de57
2022-03-02 17:35:06 [__main__] INFO: Call for function setA_Signature of first contract by second_contract is done 0xec8bb0ccdc31667bc6939a49653e68bec6e11297701a19baf9441db41bcd5293
```

### Execute tests


Tests are located in folder: 
```{ROOT_DIR}/tests/```
- test_geth_traces.py
- test_open_eth_traces.py

In CMD
``` 
 pytest tests/test_geth_traces.py --capture=tee-sys
 pytest tests/test_open_eth_traces.py --capture=tee-sys
```


### What tests do
1. Send RPC request for transaction trace (gETHa and OpenEthereum)
2. Check that response was received
3. Check that received response does not have 'error' section.
4. Check that received reply 'trace' format corresponds to RPC request node type format (by schemes)

Methods tested by automation test-framework.
<table>
<th>methods</th><th>Node</th>
    <tr>
        <td>debug_traceCall</td>
        <td>gETH</td>
    </tr>
    <tr>
        <td>debug_traceCall + js filtering</td>
        <td>gETH</td>
    </tr>
    <tr>
        <td>debug_traceTransaction</td>
        <td>gETH</td>
    </tr>
    <tr>
        <td>debug_traceTransaction + js filtering</td>
        <td>gETH</td>
    </tr>
    <tr>
        <td>trace_call</td>
        <td>OpenEthereum</td>
    </tr>
    <tr>
        <td>trace_replayBlockTransactions</td>
        <td>OpenEthereum</td>
    </tr>
    <tr>
        <td>trace_replayTransaction</td>
        <td>OpenEthereum</td>
    </tr>
    <tr>
        <td>trace_transaction</td>
        <td>OpenEthereum</td>
    </tr>
    <tr>
        <td>trace_block</td>
        <td>OpenEthereum</td>
    </tr>
</table>

(!NOTE: format check is done with validation of response scheme, 
that is collected for RPC trace methods responses from official gETH and OpenEthereum. Schemes in ```.json``` 
format can be found in folder ```{ROOT_DIR}/resources/schemas```)