from time import sleep
from unittest import TestCase

from web3 import Web3

from helpers.requests_helper import send_trace_request, request_airdrop

PROXY_URL = 'http://proxy:9090/solana'
TRACER_URL = 'http://neon-tracer:8250'
CONTRACT_CODE = '6060604052600080fd00a165627a7a72305820e75cae05548a56ec53108e39a532f0644e4b92aa900cc9f2cf98b7ab044539380029'
DEPLOY_CODE = '60606040523415600e57600080fd5b603580601b6000396000f300' + CONTRACT_CODE
proxy = Web3(Web3.HTTPProvider(PROXY_URL))
eth_account = proxy.eth.account.create('https://github.com/neonlabsorg/proxy-model.py/issues/147')
proxy.eth.default_account = eth_account.address


class TestEthGetCode(TestCase):
    @classmethod
    def setUpClass(cls):
        request_airdrop(eth_account.address)
        cls.deploy_test_contract(cls)
        # wait for a while in order to deployment to be done
        sleep(10)

    def deploy_test_contract(self):
        trx_deploy = proxy.eth.account.sign_transaction(dict(
            nonce=proxy.eth.get_transaction_count(proxy.eth.default_account),
            chainId=proxy.eth.chain_id,
            gas=987654321,
            gasPrice=2000000000,
            to='',
            value=0,
            data=bytes.fromhex(DEPLOY_CODE)),
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
        self.deploy_address = trx_deploy_receipt.contractAddress

    @staticmethod
    def get_code_ex(address, block_number):
        data = f'{{' \
               f'"jsonrpc":"2.0", ' \
               f'"method": "eth_getCode", ' \
               f'"params": ' \
               f'["{address}", {block_number}], ' \
               f'"id": 1}}'
        print('eth_getCode request data:', data)
        resp = send_trace_request(TRACER_URL, data)
        print('eth_getCode response:', resp)

        return resp.get('result')

    def get_code(self, block_number):
        return TestEthGetCode.get_code_ex(self.deploy_address, block_number)

    def test_eth_get_code(self):
        expected_code = '0x' + CONTRACT_CODE
        self.assertEqual(self.get_code(proxy.eth.block_number), expected_code)
        self.assertEqual(self.get_code(self.deploy_block_num), expected_code)
        self.assertEqual(self.get_code(self.deploy_block_num + 1), expected_code)

    def test_eth_get_code_before_deployment(self):
        self.assertEqual(self.get_code(0), '0x')
        self.assertEqual(self.get_code(self.deploy_block_num - 1), '0x')

    def test_eth_get_code_incorrect_address(self):
        self.assertEqual(
            TestEthGetCode.get_code_ex('0x71C7656EC7ab88b098defB751B7401B5f6d8976F', proxy.eth.block_number),
            '0x',
        )

