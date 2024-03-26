export const sendRPCRequest = async (method: string, params?: Params[]) => {
  let response: Response | undefined;
  let json: RpcResponse | undefined;
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

    json = (await response.json()) as RpcResponse;
  } catch (e) {
    console.log(e);
    response = response || ({ status: 500 } as Response);
  }

  return { status: response.status, payload: json };
};

export interface RpcResponse {
  jsonrpc: string;
  result: object;
  id: number;
}

type Params = string | number | boolean | object | null;
