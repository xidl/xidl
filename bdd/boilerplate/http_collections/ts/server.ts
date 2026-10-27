import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import {
  type CollectionInputs,
  CollectionInputsOperations,
} from './http_collections.server.js';

const service: CollectionInputs = {
  numbers: (count, ratio, units, threshold) => ({
    count,
    ratio,
    threshold,
    units,
  }),
  read: (values, tags, names, labels) => ({ labels, names, tags, values }),
  search: term => ({ term }),
  async *stream(values) {
    yield { values };
  },
};
const handler = createRouter(
  Object.values(CollectionInputsOperations),
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
