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

function isEIP1898Method(req) {
    let method = req.method;
    let params = req.params;

    if (method === "eth_getStorageAt") {
        return !(params[2] === "latest");
    }
    else if (method === "eth_getBalance") {
        return !(params[1] === "latest");
    }
    else if (method === "eth_getCode") {
        return !(params[1] === "latest");
    }
    else if (method === "eth_getTransactionCount") {
        return !(params[1] === "latest");
    }
    else if (method === "eth_call") {
        return !(params[1] === "latest");
    }

    return false;
}

async function process(req) {
    let json = JSON.parse(req.requestText);
    if (isTracingMethod(json)) {
        let res = await req.subrequest("/tracer", { method: "POST" });
        req.return(res.status, res.responseBody);
        return;
    }

    if (!isEIP1898Method(json)) {
        let res = await req.subrequest("/proxy", { method: "POST" });
        req.return(res.status, res.responseBody);
        return;
    }

    let res = await req.subrequest("/tracer", { method: "POST" });
    req.return(res.status, res.responseBody);
}

export default { process }