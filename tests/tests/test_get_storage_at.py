from unittest import TestCase

from solcx import install_solc
from web3 import Web3

from helpers.requests_helper import request_airdrop, send_trace_request, deploy_storage_contract

install_solc(version='0.7.6')
from solcx import compile_source
from time import sleep

PROXY_URL = "http://proxy:9090/solana"
TRACER_URL = "http://neon-tracer:8250"
proxy = Web3(Web3.HTTPProvider(PROXY_URL))
eth_account = proxy.eth.account.create('https://github.com/neonlabsorg/tracer-api/issues/3')
proxy.eth.default_account = eth_account.address

class TestGetStorageAt(TestCase):
    @classmethod
    def setUpClass(cls):
        request_airdrop(eth_account.address)
        cls.storage_contract, cls.deploy_block_num = deploy_storage_contract(proxy, eth_account)

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

    def get_storage_at(self, contract_address, index, block_number):
        data = f'{{"jsonrpc":"2.0", "method": "eth_getStorageAt", "params": ["{contract_address}","{hex(index)}",{block_number}],"id": 1}}'
        resp = send_trace_request(TRACER_URL, data)
        result = resp.get('result', None)
        self.assertTrue(result is not None)
        return int(result, base=16)

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

        self.assertEqual(self.get_storage_at(self.storage_contract.address, value_idx, block0), 0)
        self.assertEqual(self.get_storage_at(self.storage_contract.address, value_idx, block1), value1)
        self.assertEqual(self.get_storage_at(self.storage_contract.address, value_idx, block2), value2)

    def test_account_not_found(self):
        block = proxy.eth.block_number
        sleep(10)

        non_existent_account = proxy.eth.account.create("Not exist")
        self.assertEqual(self.get_storage_at(non_existent_account.address, 0, block), 0)

    def test_account_is_not_contract(self):
        block = proxy.eth.block_number
        sleep(10)

        personal_account = proxy.eth.account.create("Personal account")
        request_airdrop(personal_account.address)
        self.assertEqual(self.get_storage_at(personal_account.address, 0, block), 0)
