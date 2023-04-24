const tracingMethods = new Set([
    'debug_traceBlock',
    'debug_traceBlockByHash',
    'debug_traceBlockByNumber',
    'debug_traceBlockFromFile',
    'debug_traceCall',
    'debug_traceTransaction',
    'trace_block',
    'trace_call',
    'trace_callMany',
    'trace_filter',
    'trace_get',
    'trace_rawTransaction',
    'trace_replayBlockTransactions',
    'trace_replayTransaction',
    'trace_transaction',
]);

const eip1898Methods = {
    'eth_getStorageAt': 2,
    'eth_getBalance': 1,
    'eth_getCode': 1,
    'eth_getTransactionCount': 1,
    'eth_call': 1,
};

const predefinedTags = new Set(['latest', 'pending', 'earliest']);

function isEIP1898Method(req) {
    let paramIndex = eip1898Methods[req.method];
    return (paramIndex !== undefined) && !predefinedTags.has(req.params[paramIndex]);
}

async function process(req) {
    let json = JSON.parse(req.requestText);
    if (tracingMethods.has(json.method) || isEIP1898Method(json)) {
        req.internalRedirect('/tracer');
    } else {
        req.internalRedirect('/proxy');
    }
}

export default { process }
