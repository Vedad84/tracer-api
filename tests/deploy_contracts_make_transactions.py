import json

from common.compile import CompileSolidity
from common.web3_connection import RpcConnection
from dotenv import load_dotenv
from parse_args import cfg
import os

from utils.log_report import LogForReporter

log = LogForReporter(__name__).logger

ROOT_DIR = os.path.dirname(os.path.abspath(__file__))
RESOURCES = f'{ROOT_DIR}/resources/'

contract_address = []

transactions_to_test = {}

load_dotenv()

# Initialize connection to RPC endpoint
w3 = RpcConnection(cfg.endpoint)

# Solidity source code compilation
dapp_1 = CompileSolidity('resources/contracts/Contract_1.sol', sol_ver='0.4.18')
dapp_2 = CompileSolidity('resources/contracts/Contract_2.sol', sol_ver='0.4.18')

# Create the contracts
web3_contracts = [w3.eth.contract(abi=dapp_1('Deployed').abi,
                                  bytecode=dapp_1('Deployed').bytecode),
                  w3.eth.contract(abi=dapp_2('ExistingWithoutABI').abi,
                                  bytecode=dapp_2('ExistingWithoutABI').bytecode)
                  ]

sender_address = w3.eth.account.privateKeyToAccount(os.getenv("PRIVATE_KEY_1")).address

for n, web3_contract in enumerate(web3_contracts, start=1):
    # Submit the transaction that deploys the contract
    transaction = web3_contract.constructor().buildTransaction(
        {
            "chainId": w3.eth.chain_id,
            "from": sender_address,
            "nonce": w3.eth.getTransactionCount(sender_address)
        }
    )
    # Sign the transaction
    signed_txn = w3.eth.account.sign_transaction(transaction, private_key=os.getenv("PRIVATE_KEY_1"))
    tx = signed_txn.rawTransaction.hex()
    # Send It!
    tx_hash = w3.eth.send_raw_transaction(signed_txn.rawTransaction)
    # Wait for the transaction to be mined, and get the transaction receipt
    log.info("Waiting for transaction to finish...")
    tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

    assert tx_receipt.contractAddress

    transactions_to_test['deploy'] = tx_receipt.transactionHash.hex()

    contract_address.append(tx_receipt.contractAddress)

    log.info(f"Done! Contract deployed to {tx_receipt.contractAddress}")
    log.info(f"Contract deployment 'transactionHash': {tx_receipt.transactionHash.hex()}")

# Work with deployed contracts
contract1 = w3.eth.contract(address=contract_address[0], abi=dapp_1('Deployed').abi)
contract2 = w3.eth.contract(address=contract_address[1], abi=dapp_2('ExistingWithoutABI').abi)

# Set address of first contrat, which function will be called from second one.
l2_tx_1 = contract2.functions.ExistingWithoutABI_func(contract1.address).buildTransaction({
        "nonce":  w3.eth.getTransactionCount(sender_address)
    })
# Sign the transaction
signed_tx_1 = w3.eth.account.sign_transaction(l2_tx_1, os.getenv("PRIVATE_KEY_1"))
# Send It!
res = w3.eth.send_raw_transaction(signed_tx_1.rawTransaction)
# Wait for the transaction to be mined, and get the transaction receipt
tx_receipt = w3.eth.wait_for_transaction_receipt(res)

transactions_to_test['set_value'] = tx_receipt.transactionHash.hex()

log.info(f'Set address of first contract with ExistingWithoutABI_func : {tx_receipt.transactionHash.hex()}')

# Call for function in second that call for first contract function.
l2_tx_2 = contract2.functions.setA_Signature(1111).buildTransaction({
    'nonce': w3.eth.getTransactionCount(sender_address)})
# Sign the transaction
signed_tx_2 = w3.eth.account.sign_transaction(l2_tx_2, os.getenv("PRIVATE_KEY_1"))
# Send It!
res = w3.eth.send_raw_transaction(signed_tx_2.rawTransaction)
# Wait for the transaction to be mined, and get the transaction receipt
tx_receipt = w3.eth.wait_for_transaction_receipt(res)

transactions_to_test['call_level'] = tx_receipt.transactionHash.hex()

log.info(f'Call for function setA_Signature of first contract by second_contract is done {tx_receipt.transactionHash.hex()}')

# Save transactions to file
with open(f'{RESOURCES}/transactions_level.json', 'w') as file:
    json_data = json.dumps(transactions_to_test)
    file.write(json_data)

print("transactions_level.json:", json_data)
