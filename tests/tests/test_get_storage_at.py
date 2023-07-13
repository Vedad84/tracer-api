from unittest import TestCase
from web3 import Web3
from helpers.requests_helper import deploy_contract, STORAGE_SOLIDITY_SOURCE
from time import sleep
import os
from eth_account import Account
import secrets

NEON_URL = "http://neon-rpc:9090"
proxy = Web3(Web3.HTTPProvider(NEON_URL))

eth_account = proxy.eth.account.privateKeyToAccount(os.getenv("PRIVATE_KEY_1"))
proxy.eth.default_account = eth_account.address

class TestGetStorageAt(TestCase):
    @classmethod
    def setUpClass(cls):
        cls.storage_contract, cls.deploy_block_num = deploy_contract(proxy, eth_account, STORAGE_SOLIDITY_SOURCE)
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
        print("test_get_storage_at store_value() is done")
        sleep(30) # wait for a while to changes be applied

    def test_get_storage_at(self):
        value_idx = 0

        block0 = proxy.eth.block_number
        value1 = 452356
        self.store_value(value1)

        block1 = proxy.eth.block_number

        value2 = 234
        self.store_value(value2)

        block2 = proxy.eth.block_number
        sleep(30)
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(self.storage_contract.address, value_idx, block0), byteorder='big'), 0)
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(self.storage_contract.address, value_idx, block1), byteorder='big'), value1)
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(self.storage_contract.address, value_idx, block2), byteorder='big'), value2)

    def test_get_storage_at_blockhash(self):
        value_idx = 0

        value1 = 21255
        self.store_value(value1)

        blockhash1 = proxy.eth.get_block('latest')['hash']

        value2 = 55489
        self.store_value(value2)

        blockhash2 = proxy.eth.get_block('latest')['hash']
        sleep(30)

        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(
            self.storage_contract.address,
            value_idx,
            {
                "blockHash": blockhash1.hex(),
                "requireCanonical": False, # Should not make sense
            }),
            byteorder='big'), value1)

        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(
            self.storage_contract.address,
            value_idx,
            {
                "blockHash": blockhash2.hex(),
                "requireCanonical": True, # Should not make sense
            }),
            byteorder='big'), value2)

    def test_account_not_found(self):
        block = proxy.eth.block_number
        sleep(30)

        non_existent_account = proxy.eth.account.create("Not exist")
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(non_existent_account.address, 0, block), byteorder='big'), 0)

    def test_account_is_not_contract(self):
        block = proxy.eth.block_number
        sleep(10)


        new_key = secrets.token_hex(32)
        new_address= Account.from_key(new_key).address

        trx_deploy = proxy.eth.account.sign_transaction(dict(
            nonce=proxy.eth.get_transaction_count(proxy.eth.default_account),
            chainId=proxy.eth.chain_id,
            gas=987654321,
            gasPrice=163000000000,
            to=new_address,
            value=1,
        ),
            eth_account.key
        )
        hash = proxy.eth.send_raw_transaction(trx_deploy.rawTransaction)
        proxy.eth.wait_for_transaction_receipt(hash)
        sleep(30)

        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(new_address, 0, block), byteorder='big'), 0)
