from time import sleep
from unittest import TestCase
from web3 import Web3
from helpers.requests_helper import deploy_contract, STORAGE_SOLIDITY_SOURCE
import os

NEON_URL = "http://neon-rpc:9090"
proxy = Web3(Web3.HTTPProvider(NEON_URL))
eth_account = proxy.eth.account.privateKeyToAccount(os.getenv("PRIVATE_KEY_1"))
proxy.eth.default_account = eth_account.address

class TestEthCall(TestCase):
    @classmethod
    def setUpClass(cls):
        cls.storage_contract, cls.deploy_block_num = deploy_contract(proxy, eth_account, STORAGE_SOLIDITY_SOURCE)
        # wait for a while in order to deployment to be done
        sleep(30)

    def store_value(self, value):
        right_nonce = proxy.eth.get_transaction_count(proxy.eth.default_account)

        trx_store = self.storage_contract.functions.store(value).buildTransaction({'nonce': right_nonce})
        # print('trx_store:', trx_store)

        trx_store_signed = proxy.eth.account.sign_transaction(trx_store, eth_account.key)
        # print('trx_store_signed:', trx_store_signed)

        trx_store_hash = proxy.eth.send_raw_transaction(trx_store_signed.rawTransaction)
        # print('trx_store_hash:', trx_store_hash.hex())

        trx_store_receipt = proxy.eth.wait_for_transaction_receipt(trx_store_hash)
        # print('trx_store_receipt:', trx_store_receipt)
        print("test_eth_call store_value() is done")
        sleep(30)

    def eth_call_ex(self, address, block_number):
        abi_data = self.storage_contract.encodeABI('retrieve')
        res = proxy.eth.call({'to': address, 'data': abi_data}, block_number)
        if len(res) == 0:
            return None
        return int.from_bytes(res, byteorder='big')

    def eth_call(self, block_number):
        return self.eth_call_ex(self.storage_contract.address, block_number)

    def test_eth_call(self):
        block0 = proxy.eth.block_number
        blockhash0 = proxy.eth.get_block('latest')['hash']
        self.store_value(block0)

        block1 = proxy.eth.block_number
        blockhash1 = proxy.eth.get_block('latest')['hash']
        self.store_value(block1)

        block2 = proxy.eth.block_number
        blockhash2 = proxy.eth.get_block('latest')['hash']
        sleep(30)

        self.assertEqual(self.eth_call(block0), 0)
        self.assertEqual(self.eth_call(block1), block0)
        self.assertEqual(self.eth_call(block2), block1)

        print("blockhash0.hex(): ",blockhash0.hex() )
        self.assertEqual(self.eth_call({
            "blockHash": blockhash0.hex(),
            "requireCanonical": True
        }), 0)
        self.assertEqual(self.eth_call({
            "blockHash": blockhash1.hex(),
            "requireCanonical": True
        }), block0)
        self.assertEqual(self.eth_call({
            "blockHash": blockhash2.hex(),
            "requireCanonical": True
        }), block1)

    def test_eth_call_prior_deploy(self):
        self.assertIsNone(self.eth_call(self.deploy_block_num - 1))

    def test_eth_call_incorrect_address(self):
        block = proxy.eth.block_number
        self.store_value(block)

        self.assertIsNone(self.eth_call_ex('0x71C7656EC7ab88b098defB751B7401B5f6d8976F', block))

        # revert value in order to not break other tests
        self.store_value(0)


