from helpers import ALAN_PER_NEON
from time import sleep
from unittest import TestCase
from web3 import Web3
import os
from eth_account import Account
import secrets

NEON_URL = "http://neon-rpc:9090"
proxy = Web3(Web3.HTTPProvider(NEON_URL))

eth_account = proxy.eth.account.privateKeyToAccount(os.getenv("PRIVATE_KEY_1"))
proxy.eth.default_account = eth_account.address


class TestGetBalance(TestCase):
    @classmethod
    def setUpClass(cls):
        pass

    def transfer(self, address, sum):
        trx = dict(
            nonce=proxy.eth.get_transaction_count(proxy.eth.default_account),
            chainId=proxy.eth.chain_id,
            gas=987654321,
            gasPrice=163000000000,
            to=address,
            value=sum,
        )
        trx_signed = proxy.eth.account.sign_transaction( trx, eth_account.key)
        hash = proxy.eth.send_raw_transaction(trx_signed.rawTransaction)
        proxy.eth.wait_for_transaction_receipt(hash)


    def test_get_balance(self):

        new_key = secrets.token_hex(32)
        new_address= Account.from_key(new_key).address

        initial_balance = proxy.eth.get_balance(new_address)
        block0 = proxy.eth.block_number

        # Request 10 SOLs
        delta_balance1 = 10

        self.transfer(new_address, delta_balance1)
        sleep(10)  # wait for a while to change be applied
        block1 = proxy.eth.block_number

        # Request additional 20 SOLs
        delta_balance2 = 20
        self.transfer(new_address, delta_balance2)
        sleep(10)  # wait for a while to change be applied
        block2 = proxy.eth.block_number

        self.assertEqual(
            proxy.eth.get_balance(new_address, block0),
            initial_balance
        )
        self.assertEqual(
            proxy.eth.get_balance(new_address, block1),
            initial_balance + delta_balance1
        )
        self.assertEqual(
            proxy.eth.get_balance(new_address, block2),
            initial_balance + (delta_balance1 + delta_balance2)
        )

    def test_get_balance_blockhash(self):

        new_key = secrets.token_hex(32)
        new_address= Account.from_key(new_key).address

        initial_balance = proxy.eth.get_balance(new_address)
        blockhash0 = proxy.eth.get_block('latest')['hash']

        # Request 10 SOLs
        delta_balance1 = 40
        self.transfer(new_address, delta_balance1)
        sleep(10)  # wait for a while to change be applied
        blockhash1 = proxy.eth.get_block('latest')['hash']

        # Request additional 20 SOLs
        delta_balance2 = 30
        self.transfer(new_address, delta_balance2)
        sleep(10)  # wait for a while to change be applied
        blockhash2 = proxy.eth.get_block('latest')['hash']

        self.assertEqual(
            proxy.eth.get_balance(new_address,
                                  {
                                      "blockHash": blockhash0.hex(),
                                      "requireCanonical": True,
                                  }),
            initial_balance
        )
        self.assertEqual(
            proxy.eth.get_balance(new_address,
                                  {
                                      "blockHash": blockhash1.hex(),
                                      "requireCanonical": True,
                                  }),
            initial_balance + delta_balance1
        )
        self.assertEqual(
            proxy.eth.get_balance(new_address,
                                  {
                                      "blockHash": blockhash2.hex(),
                                      "requireCanonical": False,
                                  }),
            initial_balance + (delta_balance1 + delta_balance2)
        )

    def test_balance_not_found(self):
        block = proxy.eth.block_number
        sleep(10)

        new_key = secrets.token_hex(32)
        new_address= Account.from_key(new_key).address

        self.assertEqual(proxy.eth.get_balance(new_address, block), 0)
