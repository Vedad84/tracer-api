from unittest import TestCase
from web3 import Web3
from helpers.requests_helper import request_airdrop, deploy_contract, STORAGE_SOLIDITY_SOURCE
from time import sleep

NEON_URL = "http://neon-rpc:9090"
proxy = Web3(Web3.HTTPProvider(NEON_URL))
eth_account = proxy.eth.account.create('https://github.com/neonlabsorg/proxy-model.py/issues/147')
proxy.eth.default_account = eth_account.address

class TestGetStorageAt(TestCase):
    @classmethod
    def setUpClass(cls):
        request_airdrop(eth_account.address)
        cls.storage_contract, cls.deploy_block_num = deploy_contract(proxy, eth_account, STORAGE_SOLIDITY_SOURCE)

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

    def test_get_storage_at_blockhash(self):
        value_idx = 0

        value1 = 21255
        self.store_value(value1)

        sleep(10) # wait for a while to changes be applied
        blockhash1 = proxy.eth.get_block('latest')['hash']

        value2 = 55489
        self.store_value(value2)

        sleep(10) # wait for a while to changes be applied
        blockhash2 = proxy.eth.get_block('latest')['hash']

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
        sleep(10)

        non_existent_account = proxy.eth.account.create("Not exist")
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(non_existent_account.address, 0, block), byteorder='big'), 0)

    def test_account_is_not_contract(self):
        block = proxy.eth.block_number
        sleep(10)

        personal_account = proxy.eth.account.create("Personal account")
        request_airdrop(personal_account.address)
        self.assertEqual(int.from_bytes(proxy.eth.get_storage_at(personal_account.address, 0, block), byteorder='big'), 0)
