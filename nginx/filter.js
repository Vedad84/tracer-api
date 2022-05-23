function isTracingMethod(req) {
    let method = req.method;
    return method === "debug_traceBlockByNumber" ||
        method === "debug_traceTransaction" ||
        method === "debug_traceCall" ||
        method === "trace_filter" ||
        method === "trace_get" ||
        method === "trace_transaction" ||
        method === "trace_block" ||
        method === "trace_call" ||
        method === "trace_callMany" ||
        method === "trace_rawTransaction" ||
        method === "trace_replayTransaction" ||
        method === "trace_replayBlockTransactions";
}

function tagIsPredefined(tag) {
  return tag === 'latest' || tag === 'pending' || tag === 'earliest';
}

function isEIP1898Method(req) {
    let method = req.method;
    let params = req.params;

    if (method === "eth_getStorageAt") {
        return !tagIsPredefined(params[2]);
    }
    else if (method === "eth_getBalance") {
        return !tagIsPredefined(params[1]);
    }
    else if (method === "eth_getCode") {
        return !tagIsPredefined(params[1]);
    }
    else if (method === "eth_getTransactionCount") {
        return !tagIsPredefined(params[1]);
    }
    else if (method === "eth_call") {
        return !tagIsPredefined(params[1]);
    }

    return false;
}

async function process(req) {
    let json = JSON.parse(req.requestText);
    if (isTracingMethod(json)) {
        req.internalRedirect("/tracer");
        return;
    }

    if (!isEIP1898Method(json)) {
        req.internalRedirect("/proxy");
        return;
    }

    req.internalRedirect("/tracer");
}

export default { process }
