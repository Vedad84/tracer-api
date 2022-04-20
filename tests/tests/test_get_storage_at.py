from unittest import TestCase

from solcx import install_solc
from web3 import Web3

from helpers.requests_helper import request_airdrop, send_trace_request

install_solc(version='0.7.6')
from solcx import compile_source
from time import sleep

NEON_URL = "http://neon-rpc:9090"
proxy = Web3(Web3.HTTPProvider(NEON_URL))
eth_account = proxy.eth.account.create('https://github.com/neonlabsorg/proxy-model.py/issues/147')
proxy.eth.default_account = eth_account.address

STORAGE_SOLIDITY_SOURCE_147 = '''
pragma solidity >=0.4.0 <0.9.0;
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

class TestGetStorageAt(TestCase):
    @classmethod
    def setUpClass(cls):
        request_airdrop(eth_account.address)
        cls.deploy_storage_contract(cls)

    def deploy_storage_contract(self):
        compiled_sol = compile_source(STORAGE_SOLIDITY_SOURCE_147)
        contract_id, contract_interface = compiled_sol.popitem()
        storage = proxy.eth.contract(abi=contract_interface['abi'], bytecode=contract_interface['bin'])
        trx_deploy = proxy.eth.account.sign_transaction(dict(
            nonce=proxy.eth.get_transaction_count(proxy.eth.default_account),
            chainId=proxy.eth.chain_id,
            gas=987654321,
            gasPrice=2000000000,
            to='',
            value=0,
            data=storage.bytecode),
            eth_account.key
        )
        print('trx_deploy:', trx_deploy)
        self.trx_deploy_hash = proxy.eth.send_raw_transaction(trx_deploy.rawTransaction)
        print('trx_deploy_hash:', self.trx_deploy_hash.hex())
        trx_deploy_receipt = proxy.eth.wait_for_transaction_receipt(self.trx_deploy_hash)
        print('trx_deploy_receipt:', trx_deploy_receipt)

        self.deploy_block_hash = trx_deploy_receipt['blockHash']
        self.deploy_block_num = trx_deploy_receipt['blockNumber']
        print('deploy_block_hash:', self.deploy_block_hash)
        print('deploy_block_num:', self.deploy_block_num)

        self.storage_contract = proxy.eth.contract(
            address=trx_deploy_receipt.contractAddress,
            abi=storage.abi
        )

    def store_value(self, value):
        right_nonce = proxy.eth.get_transaction_count(proxy.eth.default_account)
        trx_store = self.storage_contract.functions.store(value).buildTransaction({'nonce': right_nonce})
        print('trx_store:', trx_store)
        trx_store_signed = proxy.eth.account.sign_transaction(trx_store, eth_account.key)
        print('trx_store_signed:', trx_store_signed)
        trx_store_hash = proxy.eth.send_raw_transaction(trx_store_signed.rawTransaction)
        print('trx_store_hash:', trx_store_hash.hex())
        trx_store_receipt = proxy.eth.wait_for_transaction_receipt(trx_store_hash)
        print('trx_store_receipt:', trx_store_receipt)

    def test_get_storage_at(self):
        value_idx = 0

        block0 = proxy.eth.block_number
        value1 = 452356
        self.store_value(value1)

        sleep(10) # wait for a while to changes be applied
        block1 = proxy.eth.block_number

        value2 = 234
        self.store_value(value2)

        sleep(10) # wait for a while to changes be applied
        block2 = proxy.eth.block_number

        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(self.storage_contract.address, value_idx, block0), byteorder='big'), 0)
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(self.storage_contract.address, value_idx, block1), byteorder='big'), value1)
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(self.storage_contract.address, value_idx, block2), byteorder='big'), value2)

    def test_account_not_found(self):
        block = proxy.eth.block_number
        sleep(10)

        non_existent_account = proxy.eth.account.create("Not exist")
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(non_existent_account.address, 0, block), byteorder='big'), 0)

    def test_account_is_not_contract(self):
        block = proxy.eth.block_number
        sleep(10)

        personal_account = proxy.eth.account.create("Personal account")
        request_airdrop(personal_account.address)
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(personal_account.address, 0, block), byteorder='big'), 0)
