import invariant from "tiny-invariant";

export const sendCustomRpc = async (url: string, method: string, params = [], verbose = false) => {
  const maxRetries = 120;
  const sleepTime = 500;

  const payload = {
    id: "1",
    jsonrpc: "2.0",
    method,
    params
  };

  for (let i = 0; i < maxRetries; i++) {
    verbose && console.log(`Waiting for node at ${url} to launch...`);

    try {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify(payload)
      });

      invariant(response.ok, `HTTP error! status: ${response.status}`);

      const resp = (await response.json()) as any;
      return resp.result as string;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, sleepTime));
    }
  }

  console.log(
    `Error fetching ${method} / ${params} from ${url} after ${
      (maxRetries * sleepTime) / 1000
    } seconds`
  );
  throw `Error sending custom RPC to ${url}`;
};
