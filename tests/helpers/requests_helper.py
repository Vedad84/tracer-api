import requests
import json
import os
from solcx import compile_source, install_solc

install_solc(version='0.7.6')

def send_trace_request(url, payload) -> dict:
    headers = {
        'Content-Type': 'application/json'
    }

    response = requests.request("POST", url, headers=headers, data=payload, allow_redirects=False)
    return json.loads(response.text)


def request_airdrop(address, amount: int = 1000):
    FAUCET_URL = os.environ.get('FAUCET_URL', 'http://faucet:3333')
    url = FAUCET_URL + '/request_neon'
    data = f'{{"wallet": "{address}", "amount": {amount}}}'
    r = requests.post(url, data=data)
    if not r.ok:
        print()
        print('Bad response:', r)
    assert(r.ok)

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
    print('trx_deploy:', trx_deploy)
    trx_deploy_hash = proxy.eth.send_raw_transaction(trx_deploy.rawTransaction)
    print('trx_deploy_hash:', trx_deploy_hash.hex())
    trx_deploy_receipt = proxy.eth.wait_for_transaction_receipt(trx_deploy_hash)
    print('trx_deploy_receipt:', trx_deploy_receipt)

    deploy_block_hash = trx_deploy_receipt['blockHash']
    deploy_block_num = trx_deploy_receipt['blockNumber']
    print('deploy_block_hash:', deploy_block_hash)
    print('deploy_block_num:', deploy_block_num)

    return (
        proxy.eth.contract(
            address=trx_deploy_receipt.contractAddress,
            abi=contract.abi
        ),
        deploy_block_num,
    )
