const tracingMethods = {
    'debug_traceBlockByHash': null,
    'debug_traceBlockByNumber': null,
    'debug_traceCall': null,
    'debug_traceTransaction': null,
    'trace_block': null,
    'trace_call': null,
    'trace_callMany': null,
    'trace_filter': null,
    'trace_get': null,
    'trace_rawTransaction': null,
    'trace_replayBlockTransactions': null,
    'trace_replayTransaction': null,
    'trace_transaction': null,
}

const eip1898Methods = {
    'eth_getStorageAt': 2,
    'eth_getBalance': 1,
    'eth_getCode': 1,
    'eth_getTransactionCount': 1,
    'eth_call': 1,
}

const predefinedTags = {
    'latest': null,
    'pending': null,
    'earliest': null,
}

function isEIP1898Method(req) {
    let paramIndex = eip1898Methods[req.method];
    return (paramIndex !== undefined) && predefinedTags[req.params[paramIndex]] === undefined;
}

async function process(req) {
    let json = JSON.parse(req.requestText);
    if (tracingMethods[json.method] !== undefined || isEIP1898Method(json)) {
        req.internalRedirect('/tracer');
    } else {
        req.internalRedirect('/proxy');
    }
}

export default { process }
