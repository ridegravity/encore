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

export async function streamInOut(
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
    async recv(): Promise<any> {
      return stream.recv();
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

export async function streamIn(
  service: string,
  endpoint: string,
  data: any
): Promise<any> {
  const source = getCurrentRequest();
  const stream = await runtime.RT.stream(service, endpoint, data, source);
  const response = new Promise(async (resolve, reject) => {
    try {
      resolve(await stream.recv());
    } catch (e) {
      reject(e);
    }
  });

  return {
    async send(msg: any) {
      stream.send(msg);
    },
    async close() {
      stream.close();
    },
    async response(): Promise<any> {
      return response;
    }
  };
}

export async function streamOut(
  service: string,
  endpoint: string,
  data: any
): Promise<any> {
  const source = getCurrentRequest();
  const stream = await runtime.RT.stream(service, endpoint, data, source);

  return {
    async recv(): Promise<any> {
      return stream.recv();
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
