export const sendRPCRequest = async (method: string, params?: any[]) => {
  let response;
  let json: RpcResponse | undefined
  try {
    response = await fetch("http://localhost:9944", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: method,
        params: params,
      }),
    });

    json = (await response.json()) as RpcResponse
  } catch (e) {
    console.log(e);
    response = { status: 500 };
  }

  return { status: response.status, payload: json };
};


export interface RpcResponse {
    jsonrpc: string;
    result: object
    id: number;
}