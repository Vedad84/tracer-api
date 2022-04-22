import json
from unittest import TestCase
from web3 import Web3
from helpers.requests_helper import request_airdrop, send_trace_request, deploy_storage_contract
import json
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
        data = {
            "jsonrpc":"2.0",
            "method":"eth_getTransactionCount",
            "params":[
                eth_account.address,
                block_num
            ],
            "id":1
        }
        resp = send_trace_request(NEON_URL, json.dumps(data))
        self.assertIsNotNone(resp.get('result', None))
        return int(resp['result'], 16)

    def test_get_transaction_count(self):
        block_num1 = proxy.eth.block_number
        sleep(10)
        self.assertEqual(self.get_transaction_count(block_num1), 0)

        deploy_storage_contract(proxy, eth_account)
        block_num2 = proxy.eth.block_number
        sleep(10)
        self.assertEqual(self.get_transaction_count(block_num2), 1)