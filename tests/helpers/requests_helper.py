import requests
import json


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
