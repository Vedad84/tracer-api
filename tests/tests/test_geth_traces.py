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
RESOURCES = f'{ROOT_DIR}/../resources/requests/gETH'
REQUESTS_TRACE_CALL = [f for f in listdir(RESOURCES) if isfile(join(RESOURCES, f)) and 'debug_trace_call' in f]
REQUESTS_TRANSACTION = [f for f in listdir(RESOURCES) if isfile(join(RESOURCES, f)) and 'debug_trace_transaction' in f]


@parameterized.expand(**level_test_parameters)
@assert_all()
def test_geth_debug_trace_call(tx_hex, url, role):
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
        "gasPrice": result.get('gasPrice'),
        "data": result.get('input'),
        "value": result.get('value')
    },
        result.get('blockNumber')]

    pprint(params)

    for file in REQUESTS_TRACE_CALL:
        with open(f'{RESOURCES}/{file}') as req:
            file_payload: dict = json.loads(req.read())
            file_payload['params'][0] = set_correct_params(file_payload['params'][0], params[0])
            file_payload['params'][1] = params[1]

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
def test_geth_debug_transaction_call(tx_hex, url, role):
    for file in REQUESTS_TRANSACTION:
        with open(f'{RESOURCES}/{file}') as req:
            file_payload: dict = json.loads(req.read())
            file_payload['params'].insert(0, tx_hex)

            payload = json.dumps(file_payload)

        response_dict = send_trace_request(url, payload)

        assert response_dict, 'There are no response for trace request'
        assert not response_dict.get('error'), 'There is error in response for trace request'

        method_scheme = f'{role}_{file}'
        print('\n' + blue_text(method_scheme), end='\n')

        pprint(response_dict)

        validate_type_by_scheme(response_dict['result'], method_scheme, 'result')
