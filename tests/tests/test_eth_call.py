from time import sleep
from unittest import TestCase

from solcx import compile_source, install_solc
from web3 import Web3

from helpers.requests_helper import send_trace_request, request_airdrop

install_solc(version='0.7.6')

PROXY_URL = "http://proxy:9090/solana"
TRACER_URL = "http://neon-tracer:8250"
proxy = Web3(Web3.HTTPProvider(PROXY_URL))
eth_account = proxy.eth.account.create('https://github.com/neonlabsorg/proxy-model.py/issues/147')
proxy.eth.default_account = eth_account.address

STORAGE_SOLIDITY_SOURCE = '''
pragma solidity >=0.4.0;
/**
 * @title Storage
 * @dev Store & retrieve value in a variable
 */
contract Storage {
    uint256 number;
    /**
     * @dev Store value in variable
     * @param num value to store
     */
    function store(uint256 num) public {
        number = num;
    }
    /**
     * @dev Return value
     * @return value of 'number'
     */
    function retrieve() public view returns (uint256){
        return number;
    }
}
'''


class TestEthCall(TestCase):
    @classmethod
    def setUpClass(cls):
        request_airdrop(eth_account.address)
        cls.deploy_storage_contract(cls)
        # wait for a while in order to deployment to be done
        sleep(10)

    def deploy_storage_contract(self):
        compiled_sol = compile_source(STORAGE_SOLIDITY_SOURCE)
        contract_id, contract_interface = compiled_sol.popitem()
        storage = proxy.eth.contract(abi=contract_interface['abi'], bytecode=contract_interface['bin'])
        trx_deploy = proxy.eth.account.sign_transaction(dict(
            nonce=proxy.eth.get_transaction_count(proxy.eth.default_account),
            chainId=proxy.eth.chain_id,
            gas=987654321,
            gasPrice=2000000000,
            to='',
            value=0,
            data=storage.bytecode),
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

        self.storage_contract = proxy.eth.contract(
            address=trx_deploy_receipt.contractAddress,
            abi=storage.abi
        )

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

    def eth_call_ex(self, address, block_number):
        abi_data = self.storage_contract.encodeABI('retrieve')
        data = f'{{' \
               f'"jsonrpc":"2.0", ' \
               f'"method": "eth_call", ' \
               f'"params": ' \
               f'[{{"to": "{address}", "data": "{abi_data}"}}, {block_number}], ' \
               f'"id": 1}}'
        print('eth_call request data:', data)
        resp = send_trace_request(TRACER_URL, data)
        print('eth_call response:', resp)

        result = resp.get('result', None)
        self.assertTrue(result is not None)

        exit_reason = result.get('exit_reason', None)
        self.assertTrue(exit_reason is not None)

        succeed_status = exit_reason.get('Succeed', None)
        self.assertTrue(succeed_status is not None)

        if succeed_status != 'Returned':
            return None

        result = result.get('result', None)
        self.assertTrue(result is not None)

        return int(result, base=16)

    def eth_call(self, block_number):
        return self.eth_call_ex(self.storage_contract.address, block_number)

    def test_eth_call(self):
        block0 = proxy.eth.block_number
        self.store_value(block0)

        # wait for a while in order to changes to be applied
        sleep(10)

        block1 = proxy.eth.block_number

        self.store_value(block1)

        # wait for a while in order to changes to be applied
        sleep(10)

        block2 = proxy.eth.block_number

        self.assertEqual(self.eth_call(block0), 0)
        self.assertEqual(self.eth_call(block1), block0)
        self.assertEqual(self.eth_call(block2), block1)

    def test_eth_call_prior_deploy(self):
        self.assertIsNone(self.eth_call(self.deploy_block_num - 1))

    def test_eth_call_incorrect_address(self):
        self.store_value(proxy.eth.block_number)

        # wait for a while in order to changes to be applied
        sleep(10)

        self.assertIsNone(self.eth_call_ex('0x71C7656EC7ab88b098defB751B7401B5f6d8976F', proxy.eth.block_number))

        # revert value in order to not break other tests
        self.store_value(0)

        # wait for a while in order to changes to be applied
        sleep(10)
