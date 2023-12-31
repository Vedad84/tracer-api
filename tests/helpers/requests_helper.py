import requests
import json
import os
from solcx import compile_source, install_solc

install_solc(version='0.7.6')

def set_correct_params(resource, params_from_response) -> dict:
    target_values = list(params_from_response.keys())
    return {
        k: params_from_response[k]
        for k in resource.keys()
        if k in target_values
    }


def send_trace_request(url, payload) -> dict:
    headers = {
        'Content-Type': 'application/json'
    }

    response = requests.request("POST", url, headers=headers, data=payload, allow_redirects=False)
    return json.loads(response.text)

def get_tx_info(tx_hex) -> str:
    return json.dumps({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionByHash",
        "params": [tx_hex],
        "id": 1
    })


STORAGE_SOLIDITY_SOURCE = '''
pragma solidity >=0.4.0;
/**
 * @title Storage
 * @dev Store & retrieve value in a variable
 */
contract Storage {
    uint256 number;
    /**
     * @dev Store value in variable
     * @param num value to store
     */
    function store(uint256 num) public {
        number = num;
    }
    /**
     * @dev Return value
     * @return value of 'number'
     */
    function retrieve() public view returns (uint256){
        return number;
    }
}
'''

def deploy_contract(proxy, deployer, contract_source):
    compiled_sol = compile_source(contract_source)
    contract_id, contract_interface = compiled_sol.popitem()
    contract = proxy.eth.contract(abi=contract_interface['abi'], bytecode=contract_interface['bin'])
    trx_deploy = proxy.eth.account.sign_transaction(dict(
        nonce=proxy.eth.get_transaction_count(deployer.address),
        chainId=proxy.eth.chain_id,
        gas=987654321,
        gasPrice=163000000000,
        to='',
        value=0,
        data=contract.bytecode),
        deployer.key
    )
    # print('\nrx_deploy:', trx_deploy)
    trx_deploy_hash = proxy.eth.send_raw_transaction(trx_deploy.rawTransaction)
    # print('trx_deploy_hash:', trx_deploy_hash.hex())
    trx_deploy_receipt = proxy.eth.wait_for_transaction_receipt(trx_deploy_hash)
    # print('trx_deploy_receipt:', trx_deploy_receipt)

    deploy_block_hash = trx_deploy_receipt['blockHash']
    deploy_block_num = trx_deploy_receipt['blockNumber']
    # print('deploy_block_hash:', deploy_block_hash)
    print('\ndeploy_block_num:', deploy_block_num)
    print("contract is deployed:", trx_deploy_receipt.contractAddress)

    return (
        proxy.eth.contract(
            address=trx_deploy_receipt.contractAddress,
            abi=contract.abi
        ),
        deploy_block_num,
    )
