import requests
import json
import os

def set_correct_params(resource, params_from_response) -> dict:
    target_values = list(params_from_response.keys())
    return {
        k: params_from_response[k]
        for k in resource.keys()
        if k in target_values
    }


def send_trace_request(url, payload) -> dict:
    headers = {
        'Content-Type': 'application/json'
    }

    response = requests.request("POST", url, headers=headers, data=payload, allow_redirects=False)

    return json.loads(response.text)


def get_tx_info(tx_hex) -> dict:
    return json.dumps({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionByHash",
        "params": [tx_hex],
        "id": 1
    })


def request_airdrop(address, amount: int = 10):
    FAUCET_URL = os.environ.get('FAUCET_URL', 'http://faucet:3333')
    url = FAUCET_URL + '/request_neon'
    data = f'{{"wallet": "{address}", "amount": {amount}}}'
    r = requests.post(url, data=data)
    if not r.ok:
        print()
        print('Bad response:', r)
    assert(r.ok)
