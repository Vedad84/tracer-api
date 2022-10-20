from unittest import TestCase
from web3 import Web3
from helpers.requests_helper import request_airdrop, deploy_contract
from time import sleep
import random

NEON_URL = 'http://neon-rpc:9090'
proxy = Web3(Web3.HTTPProvider(NEON_URL))
eth_account = proxy.eth.account.create('TestGetLogsAccount')
proxy.eth.default_account = eth_account.address

TEST_CONTRACT_CODE = """
pragma solidity >=0.4.0;

contract TestEvents {
    uint256 number;
    
    event TestEvent(address indexed from, uint256 value);
    
    function emitEvent(uint256 amount) public {
        number = amount;
        emit TestEvent(msg.sender, amount);
    }
}
"""

TEST_EVENT_TOPIC='0x2a1343a7ef16865394327596242ebb1d13cafbd9dbb29027e89cbc0212cfa737'

class TestGetLogs(TestCase):
    @classmethod
    def setUpClass(cls):
        request_airdrop(eth_account.address)
        cls.test_contract, cls.deploy_block_num = deploy_contract(proxy, eth_account, TEST_CONTRACT_CODE)
        sleep(10)

    def call_test_function(self, value, caller):
        nonce = proxy.eth.get_transaction_count(caller.address)
        trx_event = self.test_contract.functions.emitEvent(value).buildTransaction({ 'nonce': nonce })
        print('trx_event:', trx_event)
        trx_event_signed = proxy.eth.account.sign_transaction(trx_event, caller.key)
        print('trx_event_signed:', trx_event_signed)
        trx_event_hash = proxy.eth.send_raw_transaction(trx_event_signed.rawTransaction)
        print('trx_event_hash:', trx_event_hash.hex())
        trx_event_receipt = proxy.eth.wait_for_transaction_receipt(trx_event_hash.hex())
        print('trx_event_receipt:', trx_event_receipt)
        return trx_event_receipt

    def test_get_logs(self):
        test_acc = proxy.eth.account.create('TestAccount1')
        request_airdrop(test_acc.address)
        receipt = self.call_test_function(random.randint(1, 100000), test_acc)
        sleep(2)

        logs = proxy.eth.get_logs({
            'fromBlock': receipt['blockNumber'],
            'toBlock': receipt['blockNumber'],
            'address': self.test_contract.address,
            'topics': [TEST_EVENT_TOPIC],
        })[0]

        print(f"LOGS = {logs}")
        self.assertEqual(logs['transactionHash'], receipt['transactionHash'])
        self.assertEqual(logs['blockNumber'], receipt['blockNumber'])
        self.assertEqual(logs['address'], receipt['to'])

    def test_get_logs_by_block_hash(self):
        test_acc = proxy.eth.account.create('TestAccount1')
        request_airdrop(test_acc.address)
        receipt = self.call_test_function(random.randint(1, 100000), test_acc)
        sleep(2)

        logs = proxy.eth.get_logs({
            'blockHash': receipt['blockHash'].hex(),
            'address': self.test_contract.address,
            'topics': [TEST_EVENT_TOPIC],
        })[0]

        print(f"LOGS1 = {logs}")
        self.assertEqual(logs['transactionHash'], receipt['transactionHash'])
        self.assertEqual(logs['blockNumber'], receipt['blockNumber'])
        self.assertEqual(logs['address'], receipt['to'])

    def test_get_logs_empty_result_when_block_order_wrong(self):
        test_acc = proxy.eth.account.create('TestAccount1')
        request_airdrop(test_acc.address)
        receipt = self.call_test_function(random.randint(1, 100000), test_acc)
        sleep(2)

        logs = proxy.eth.get_logs({
            'fromBlock': receipt['blockNumber'],
            'toBlock': receipt['blockNumber'] - 1, # to_block < from_block
            'address': self.test_contract.address,
            'topics': [TEST_EVENT_TOPIC],
        })

        self.assertEqual(len(logs), 0)

    def test_get_logs_nested_topics(self):
        test_acc = proxy.eth.account.create('TestAccount1')
        request_airdrop(test_acc.address)
        receipt = self.call_test_function(random.randint(1, 100000), test_acc)
        sleep(2)

        logs = proxy.eth.get_logs({
            'fromBlock': receipt['blockNumber'],
            'toBlock': receipt['blockNumber'],
            'address': self.test_contract.address,
            'topics': ['0x0000000000000000000000000000000000000000000000000000000000000011', # <-- some non-existent topic
                       ['0x0000000000000000000000000000000000000000000000000000000000000012', TEST_EVENT_TOPIC]], # <-- nested topics
        })[0]

        print(f"LOGS = {logs}")
        self.assertEqual(logs['transactionHash'], receipt['transactionHash'])
        self.assertEqual(logs['blockNumber'], receipt['blockNumber'])
        self.assertEqual(logs['address'], receipt['to'])
