import * as runtime from "../runtime/mod";
import { getCurrentRequest } from "../reqtrack/mod";

export async function apiCall(
  service: string,
  endpoint: string,
  data: any
): Promise<any> {
  const source = getCurrentRequest();
  return runtime.RT.apiCall(service, endpoint, data, source);
}
export async function stream(
  service: string,
  endpoint: string,
  data: any
): Promise<any> {
  const source = getCurrentRequest();
  const stream = await runtime.RT.stream(service, endpoint, data, source);

  return {
    async send(msg: any) {
      stream.send(msg);
    },
    async recv() {
      stream.recv();
    },
    async close() {
      stream.close();
    },
    async *[Symbol.asyncIterator]() {
      while (true) {
        try {
          yield await stream.recv();
        } catch (e) {
          break;
        }
      }
    }
  };
}
