from unittest import TestCase
from web3 import Web3
from helpers.requests_helper import  deploy_contract, STORAGE_SOLIDITY_SOURCE
from time import sleep
import os

NEON_URL = "http://neon-rpc:9090"
proxy = Web3(Web3.HTTPProvider(NEON_URL))

eth_account = proxy.eth.account.privateKeyToAccount(os.getenv("PRIVATE_KEY_1"))
proxy.eth.default_account = eth_account.address

class TestGetTransactionCount(TestCase):
    @classmethod
    def setUpClass(cls):
        pass

    def get_transaction_count(self, block_num):
        res = proxy.eth.get_transaction_count(eth_account.address, block_num)
        print(f'get_transaction_count = {res}')
        return res

    def test_get_transaction_count(self):
        block_num1 = proxy.eth.block_number
        sleep(30)
        nonce = self.get_transaction_count(block_num1);


        deploy_contract(proxy, eth_account, STORAGE_SOLIDITY_SOURCE)
        block_num2 = proxy.eth.block_number
        block_hash2 = proxy.eth.get_block('latest')['hash']
        sleep(30)
        self.assertEqual(self.get_transaction_count(block_num2), nonce+1)
        self.assertEqual(self.get_transaction_count({
            "blockHash": block_hash2.hex(),
            "requireCanonical": True
        }), nonce + 1)