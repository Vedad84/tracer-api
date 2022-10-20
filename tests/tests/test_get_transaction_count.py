from unittest import TestCase
from web3 import Web3
from helpers.requests_helper import request_airdrop, deploy_contract, STORAGE_SOLIDITY_SOURCE
from time import sleep

NEON_URL = "http://neon-rpc:9090"
proxy = Web3(Web3.HTTPProvider(NEON_URL))
eth_account = proxy.eth.account.create('https://github.com/neonlabsorg/tracer-api/issues/5')
proxy.eth.default_account = eth_account.address

class TestGetTransactionCount(TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        request_airdrop(eth_account.address)

    def get_transaction_count(self, block_num):
        res = proxy.eth.get_transaction_count(eth_account.address, block_num)
        print(f'get_transaction_count = {res}')
        return res

    def test_get_transaction_count(self):
        block_num1 = proxy.eth.block_number
        block_hash1 = proxy.eth.get_block('latest')['hash']
        sleep(10)
        self.assertEqual(self.get_transaction_count(block_num1), 0)
        self.assertEqual(self.get_transaction_count({
            "blockHash": block_hash1.hex(),
            "requireCanonical": True
        }), 0)

        deploy_contract(proxy, eth_account, STORAGE_SOLIDITY_SOURCE)
        block_num2 = proxy.eth.block_number
        block_hash2 = proxy.eth.get_block('latest')['hash']
        sleep(10)
        self.assertEqual(self.get_transaction_count(block_num2), 1)
        self.assertEqual(self.get_transaction_count({
            "blockHash": block_hash2.hex(),
            "requireCanonical": True
        }), 1)