import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import {
  type RequiredParameters,
  RequiredParametersOperations,
} from './required_parameters.server.js';

const inputs = (count: number, active: boolean, label: string) => ({
  active,
  count,
  label,
});
const service: RequiredParameters = {
  cookie: inputs,
  header: inputs,
  optional: (count, active, label) =>
    inputs(count ?? 0, active ?? false, label ?? ''),
  query: inputs,
  async *stream(count, active, label) {
    yield inputs(count, active, label);
  },
};
const handler = createRouter(
  Object.values(RequiredParametersOperations),
  service,
);
createServer(async (request, response) => {
  const headers = new Headers();
  for (const [name, value] of Object.entries(request.headers)) {
    for (const part of Array.isArray(value)
      ? value
      : value === undefined
        ? []
        : [value]) {
      headers.append(name, part);
    }
  }
  const result = await handler(
    new Request(`http://${request.headers.host}${request.url}`, { headers }),
  );
  response.writeHead(result.status, Object.fromEntries(result.headers));
  response.end(Buffer.from(await result.arrayBuffer()));
}).listen(Number(process.env.PORT), '127.0.0.1');
