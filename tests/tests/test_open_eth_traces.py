from pprint import pprint
import json
import os
from os import listdir
from os.path import isfile, join
from helpers.requests_helper import set_correct_params, send_trace_request, get_tx_info
from parameterized import parameterized
from helpers.soft_assertion import assert_all
from helpers.test_helper import validate_type_by_scheme
from helpers.testing_parameterization import level_test_parameters
from helpers import blue_text

ROOT_DIR = os.path.dirname(os.path.abspath(__file__))
RESOURCES = f'{ROOT_DIR}/../resources/requests/open_eth'
REQUESTS_TRACE_CALL = [f for f in listdir(RESOURCES) if isfile(join(RESOURCES, f)) and 'trace_call' in f]
REQUESTS_TRANSACTION = [f for f in listdir(RESOURCES) if isfile(join(RESOURCES, f)) and 'transaction.json' in f]
REQUEST_BLOCK = [f for f in listdir(RESOURCES) if isfile(join(RESOURCES, f)) and 'block' in f]


@parameterized.expand(**level_test_parameters)
@assert_all()
def test_open_eth_trace_call(tx_hex, url, role):
    # get_tx_info = json.dumps({
    #     "jsonrpc": "2.0",
    #     "method": "eth_getTransactionByHash",
    #     "params": [tx_hex],
    #     "id": 1
    # })

    tx_info_response_dict = send_trace_request('http://proxy:9090/solana', get_tx_info(tx_hex))

    result = tx_info_response_dict['result']
    params = [{
        "from": result.get('from'),
        "gas": result.get('gas'),
        "to": result.get('to'),
        "nonce": result.get('nonce'),
        "gasPrice": result.get('gasPrice'),
        "data": result.get('input'),
        "value": result.get('value')
    }
    ]

    pprint(params)

    for file in REQUESTS_TRACE_CALL:
        with open(f'{RESOURCES}/{file}') as req:
            file_payload: dict = json.loads(req.read())
            file_payload['params'][0] = set_correct_params(file_payload['params'][0], params[0])

            payload = json.dumps(file_payload)

        response_dict = send_trace_request(url, payload)

        assert response_dict, 'There are no response for trace request'
        assert not response_dict.get('error'), 'There is error in response for trace request'

        method_scheme = f'{role}_{file}'
        print('\n' + blue_text(method_scheme), end='\n')

        pprint(response_dict)

        validate_type_by_scheme(response_dict['result'], method_scheme, 'result')


@parameterized.expand(**level_test_parameters)
@assert_all()
def test_open_eth_transaction_call(tx_hex, url, role):
    for file in REQUESTS_TRANSACTION:
        with open(f'{RESOURCES}/{file}') as req:
            file_payload: dict = json.loads(req.read())
            file_payload['params'][0] = tx_hex

            payload = json.dumps(file_payload)

        response_dict = send_trace_request(url, payload)

        assert response_dict, 'There are no response for trace request'
        assert not response_dict.get('error'), 'There is error in response for trace request'

        method_scheme = f'{role}_{file}'
        print('\n' + blue_text(method_scheme), end='\n')

        pprint(response_dict)

        validate_type_by_scheme(response_dict['result'], method_scheme, 'result')


@parameterized.expand(**level_test_parameters)
@assert_all()
def test_open_eth_block_call(tx_hex, url, role):

    tx_info_response_dict = send_trace_request('http://proxy:9090/solana', get_tx_info(tx_hex))

    for file in REQUEST_BLOCK:
        with open(f'{RESOURCES}/{file}') as req:
            file_payload: dict = json.loads(req.read())
            file_payload['params'][0] = tx_info_response_dict['result']['blockNumber']

            payload = json.dumps(file_payload)

        response_dict = send_trace_request(url, payload)

        assert response_dict, 'There are no response for trace request'
        assert not response_dict.get('error'), 'There is error in response for trace request'

        method_scheme = f'{role}_{file}'
        print('\n' + blue_text(method_scheme), end='\n')

        pprint(response_dict)

        validate_type_by_scheme(response_dict['result'], method_scheme, 'result')

