from helpers import ALAN_PER_NEON
from helpers.requests_helper import request_airdrop, send_trace_request
from time import sleep
from unittest import TestCase
from web3 import Web3

NEON_URL = "http://neon-rpc:9090"
proxy = Web3(Web3.HTTPProvider(NEON_URL))
eth_account = proxy.eth.account.create('https://github.com/neonlabsorg/proxy-model.py/issues/147')
proxy.eth.default_account = eth_account.address


class TestGetBalance(TestCase):
    @classmethod
    def setUpClass(cls):
        pass

    def test_get_balance(self):
        initial_balance = proxy.eth.get_balance(eth_account.address)
        block0 = proxy.eth.block_number

        # Request 10 SOLs
        delta_balance1 = 10
        request_airdrop(eth_account.address, delta_balance1)
        sleep(10)  # wait for a while to change be applied
        block1 = proxy.eth.block_number

        # Request additional 20 SOLs
        delta_balance2 = 20
        request_airdrop(eth_account.address, delta_balance2)
        sleep(10)  # wait for a while to change be applied
        block2 = proxy.eth.block_number

        self.assertEqual(
            proxy.eth.get_balance(eth_account.address, block0),
            initial_balance
        )
        self.assertEqual(
            proxy.eth.get_balance(eth_account.address, block1),
            initial_balance + delta_balance1 * ALAN_PER_NEON
        )
        self.assertEqual(
            proxy.eth.get_balance(eth_account.address, block2),
            initial_balance + (delta_balance1 + delta_balance2) * ALAN_PER_NEON
        )

    def test_get_balance_blockhash(self):
        initial_balance = proxy.eth.get_balance(eth_account.address)
        blockhash0 = proxy.eth.get_block('latest')['hash']

        # Request 10 SOLs
        delta_balance1 = 40
        request_airdrop(eth_account.address, delta_balance1)
        sleep(10)  # wait for a while to change be applied
        blockhash1 = proxy.eth.get_block('latest')['hash']

        # Request additional 20 SOLs
        delta_balance2 = 30
        request_airdrop(eth_account.address, delta_balance2)
        sleep(10)  # wait for a while to change be applied
        blockhash2 = proxy.eth.get_block('latest')['hash']

        self.assertEqual(
            proxy.eth.get_balance(eth_account.address,
                                  {
                                      "blockHash": blockhash0.hex(),
                                      "requireCanonical": True,
                                  }),
            initial_balance
        )
        self.assertEqual(
            proxy.eth.get_balance(eth_account.address,
                                  {
                                      "blockHash": blockhash1.hex(),
                                      "requireCanonical": True,
                                  }),
            initial_balance + delta_balance1 * ALAN_PER_NEON
        )
        self.assertEqual(
            proxy.eth.get_balance(eth_account.address,
                                  {
                                      "blockHash": blockhash2.hex(),
                                      "requireCanonical": False,
                                  }),
            initial_balance + (delta_balance1 + delta_balance2) * ALAN_PER_NEON
        )

    def test_balance_not_found(self):
        block = proxy.eth.block_number
        sleep(10)

        non_existent_account = proxy.eth.account.create("Not exist")
        self.assertEqual(proxy.eth.get_balance(non_existent_account.address, block), 0)
