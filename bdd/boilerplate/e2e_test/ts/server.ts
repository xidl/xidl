import { createServer } from 'node:http';
import type {
  E2EHttpFormSubmitProfileResponse,
  E2EHttpRouteAndBodyGetMsgpackResourceResponse,
  E2EHttpScopeMatrixOverrideBothMediaResponse,
  E2EHttpScopeMatrixOverrideConsumesOnlyResponse,
  E2EHttpScopeMatrixOverrideProducesOnlyResponse,
  E2ETypeServerParameterOp3Response,
  E2ETypeServerParameterOp4Response,
  E2ETypeServerParameterOp5Response,
  E2ETypeServerParameterOp6Response,
} from './e2e_test.iface.js';
import type {
  EnumEmpty,
  EnumSimple1,
  StructEmpty,
  StructHttpBody,
  StructSimple,
  UnionSimple,
} from './e2e_test.js';
import {
  createE2eAttributeHandler,
  createE2eHttpDefaultsMatrixHandler,
  createE2eHttpFormHandler,
  createE2eHttpRouteAndBodyHandler,
  createE2eHttpScopeMatrixHandler,
  createE2eHttpSecurityHandler,
  createE2eHttpSecurityMatrixHandler,
  createE2ePathSeverHandler,
  createE2eTypeServerHandler,
  type E2eAttribute,
  type E2eHttpDefaultsMatrix,
  type E2eHttpForm,
  type E2eHttpRouteAndBody,
  type E2eHttpScopeMatrix,
  type E2eHttpSecurity,
  type E2eHttpSecurityMatrix,
  type E2ePathSever,
  type E2eTypeServer,
} from './e2e_test.server.js';

function formatOpt(v: string | null | undefined): string {
  if (v === undefined || v === null) return 'None';
  return `Some("${v}")`;
}

function formatOptInt(v: number | null | undefined): string {
  if (v === undefined || v === null) return 'None';
  return `Some(${v})`;
}

class MyE2ePathSever implements E2ePathSever {
  async op_with_path(req: { param1: string }): Promise<string[]> {
    return [req.param1];
  }
  async op_with_query(req: { param1: string; q: string }): Promise<string[]> {
    return [req.param1, req.q];
  }
  async op_with_params(req: {
    path_name: string;
    q: string[];
    b: Uint8Array;
    a: Record<string, any>;
  }): Promise<string[]> {
    const res = [req.path_name, ...req.q];
    res.push(JSON.stringify(Array.from(req.b)));
    res.push(JSON.stringify(req.a));
    return res;
  }
  async op_with_query2(req: {
    all: string;
    word: string;
    q: string;
  }): Promise<string> {
    return `${req.all}:${req.word}:${req.q}`;
  }
}

class MyE2eHttpRouteAndBody implements E2eHttpRouteAndBody {
  async get_resource(req: {
    resource_id: string;
    locale?: string;
    trace_id: string;
  }): Promise<string> {
    return `id:${req.resource_id},lang:${formatOpt(req.locale)},trace:${req.trace_id}`;
  }
  async get_file(req: {
    file_path: string;
    download: boolean;
    version?: string;
  }): Promise<string> {
    let filePath = req.file_path;
    if (filePath.startsWith('/')) {
      filePath = filePath.slice(1);
    }
    return `file:${filePath},download:${req.download},version:${formatOpt(req.version)}`;
  }
  async create_resource(req: {
    resource_body: StructHttpBody;
  }): Promise<StructHttpBody> {
    return req.resource_body;
  }
  async replace_resource(): Promise<void> {}
  async patch_resource(req: {
    changes: Record<string, any>;
  }): Promise<Record<string, any>> {
    return req.changes;
  }
  async delete_resource(): Promise<void> {}
  async probe_resource(): Promise<void> {}
  async resource_options(): Promise<void> {}
  async get_msgpack_resource(): Promise<E2EHttpRouteAndBodyGetMsgpackResourceResponse> {
    return {
      return: { labels: {}, name: 'msgpack', tags: [] },
      revision: 1,
    };
  }
  async dedup_resource(req: {
    id: string;
    x_trace_id: string;
  }): Promise<string> {
    return `${req.id}:${req.x_trace_id}`;
  }
  async preview_resource(req: {
    resource: StructHttpBody;
  }): Promise<StructHttpBody> {
    return req.resource;
  }
}

class MyE2eHttpSecurity implements E2eHttpSecurity {
  async get_secure_user(req: {
    user_id: string;
    locale?: string;
    trace_id: string;
  }): Promise<string> {
    return `user:${req.user_id},lang:${formatOpt(req.locale)},trace:${req.trace_id}`;
  }
  async search_secure_user(req: {
    keyword: string;
    page?: number;
  }): Promise<string> {
    return `keyword:${req.keyword},page:${formatOptInt(req.page)}`;
  }
  async healthz(): Promise<string> {
    return 'ok';
  }
}

class MyE2eTypeServer implements E2eTypeServer {
  async get_attribute_type_attr1(): Promise<string> {
    return typeServerState.type_attr1;
  }
  async set_attribute_type_attr1(req: { value: string }): Promise<void> {
    typeServerState.type_attr1 = req.value;
  }
  async get_attribute_type_attr2(): Promise<string[]> {
    return typeServerState.type_attr2;
  }
  async simple_op(): Promise<void> {}
  async simple_op_with_return1(): Promise<string> {
    return 'simple_op_with_return1';
  }
  async simple_op_with_return2(): Promise<any> {
    return {};
  }
  async simple_op_with_return3(): Promise<EnumSimple1> {
    return 'V1' as EnumSimple1;
  }
  async simple_op_with_return4(): Promise<StructEmpty> {
    return {};
  }
  async simple_op_with_return5(): Promise<any> {
    return {};
  }
  async return_with_sequence1(): Promise<string[]> {
    return ['s1', 's2'];
  }
  async return_with_sequence2(): Promise<any[]> {
    return [];
  }
  async return_with_sequence3(): Promise<EnumSimple1[]> {
    return ['V1' as EnumSimple1, 'V2' as EnumSimple1];
  }
  async return_with_sequence4(): Promise<StructEmpty[]> {
    return [{}];
  }
  async return_with_sequence5(): Promise<any[]> {
    return [];
  }
  async return_with_map(): Promise<Record<string, number>> {
    return { k1: 1 };
  }
  async return_with_any(): Promise<any> {
    return { any: 'value' };
  }
  async return_with_any_sequence(): Promise<any[]> {
    return [1, 'two'];
  }
  async return_with_any_map(): Promise<Record<string, any>> {
    return { k1: 1 };
  }
  async parameter_op(): Promise<void> {}
  async parameter_op2(): Promise<void> {}
  async parameter_op3(): Promise<E2ETypeServerParameterOp3Response> {
    return { b: 3, c: [] };
  }
  async parameter_op4(): Promise<E2ETypeServerParameterOp4Response> {
    return { a: 'op4', b: 4, c: [] };
  }
  async parameter_op5(): Promise<E2ETypeServerParameterOp5Response> {
    return { a: 'op5', b: 5, c: [], return: ['op5'] };
  }
  async parameter_op6(): Promise<E2ETypeServerParameterOp6Response> {
    return { a: 'op6', b: 6, c: [], return: {} };
  }
}

class MyE2eAttribute implements E2eAttribute {
  async get_attribute_attr1(): Promise<string> {
    return attributeState.attr1;
  }
  async set_attribute_attr1(req: { value: string }): Promise<void> {
    attributeState.attr1 = req.value;
  }
  async get_attribute_attr2(): Promise<string[]> {
    return attributeState.attr2;
  }
  async get_attribute_attr3(): Promise<EnumEmpty> {
    return {} as EnumEmpty;
  }
  async set_attribute_attr3(): Promise<void> {}
  async get_attribute_attr4(): Promise<EnumSimple1> {
    return attributeState.attr4 as EnumSimple1;
  }
  async set_attribute_attr4(req: { value: EnumSimple1 }): Promise<void> {
    attributeState.attr4 = req.value;
  }
  async get_attribute_attr5(): Promise<StructEmpty> {
    return {};
  }
  async set_attribute_attr5(): Promise<void> {}
  async get_attribute_attr6(): Promise<StructSimple> {
    return {
      member1: {} as EnumEmpty,
      member2: 'V1' as EnumSimple1,
      member3: {},
    };
  }
  async set_attribute_attr6(): Promise<void> {}
  async get_attribute_attr61(): Promise<UnionSimple> {
    return attributeState.attr61 as UnionSimple;
  }
  async set_attribute_attr61(req: { value: UnionSimple }): Promise<void> {
    attributeState.attr61 = req.value as any;
  }
  async get_attribute_attr7(): Promise<string[]> {
    return [];
  }
  async set_attribute_attr7(): Promise<void> {}
  async get_attribute_attr8(): Promise<EnumEmpty[]> {
    return [];
  }
  async set_attribute_attr8(): Promise<void> {}
  async get_attribute_attr9(): Promise<EnumSimple1[]> {
    return [];
  }
  async set_attribute_attr9(): Promise<void> {}
  async get_attribute_attr10(): Promise<StructEmpty[]> {
    return [];
  }
  async set_attribute_attr10(): Promise<void> {}
  async get_attribute_attr11(): Promise<StructSimple[]> {
    return [];
  }
  async set_attribute_attr11(): Promise<void> {}
  async get_attribute_attr12(): Promise<Record<string, number>> {
    return {};
  }
  async set_attribute_attr12(): Promise<void> {}
  async get_attribute_attr13(): Promise<any> {
    return null;
  }
  async set_attribute_attr13(): Promise<void> {}
  async get_attribute_attr14(): Promise<any[]> {
    return [];
  }
  async set_attribute_attr14(): Promise<void> {}
  async get_attribute_attr15(): Promise<Record<string, any>> {
    return {};
  }
  async set_attribute_attr15(): Promise<void> {}
  async get_attribute_attr16(): Promise<string> {
    return 'attr16';
  }
}

class MyE2eHttpForm implements E2eHttpForm {
  async submit_profile(req: {
    name: string;
    age?: number;
  }): Promise<E2EHttpFormSubmitProfileResponse> {
    return {
      normalized_name: req.name.toUpperCase(),
      return: `name:${req.name},age:${formatOptInt(req.age)}`,
    };
  }
}

class MyE2eHttpScopeMatrix implements E2eHttpScopeMatrix {
  async get_attribute_scope_inherited_attr(): Promise<string> {
    return 'inherited';
  }
  async get_attribute_scope_bare_attr(): Promise<string> {
    return 'bare';
  }
  async default_scope(req: { request_body: StructHttpBody }): Promise<string> {
    return req.request_body.name;
  }
  async override_consumes_only(req: {
    name: string;
    age?: number;
  }): Promise<E2EHttpScopeMatrixOverrideConsumesOnlyResponse> {
    return {
      normalized_name: req.name.toUpperCase(),
      return: `name:${req.name},age:${formatOptInt(req.age)}`,
    };
  }
  async override_produces_only(req: {
    resource_id: string;
  }): Promise<E2EHttpScopeMatrixOverrideProducesOnlyResponse> {
    return {
      return: { labels: {}, name: req.resource_id, tags: [] },
      revision: 1,
    };
  }
  async override_both_media(req: {
    name: string;
    age?: number;
  }): Promise<E2EHttpScopeMatrixOverrideBothMediaResponse> {
    return {
      normalized_name: 'OVERRIDDEN',
      return: {
        labels: {},
        name: req.name,
        tags: [`age:${formatOptInt(req.age)}`],
      },
    };
  }
  async deprecated_plain(req: { resource_id: string }): Promise<string> {
    return req.resource_id;
  }
  async deprecated_since_only(req: { resource_id: string }): Promise<string> {
    return req.resource_id;
  }
  async deprecated_window(req: { resource_id: string }): Promise<string> {
    return req.resource_id;
  }
}

class MyE2eHttpDefaultsMatrix implements E2eHttpDefaultsMatrix {
  async delete_resource_default_query(req: {
    id: string;
    revision: number;
  }): Promise<string> {
    return `${req.id}:${req.revision}`;
  }
  async probe_resource_default_query(): Promise<void> {}
  async resource_options_default_query(): Promise<void> {}
  async replace_resource_default_body(req: {
    id: string;
    name: string;
    alias?: string;
  }): Promise<StructHttpBody> {
    return { alias: req.alias, labels: {}, name: req.name, tags: [req.id] };
  }
  async patch_resource_default_body(req: {
    id: string;
    name: string;
    alias?: string;
  }): Promise<StructHttpBody> {
    return { alias: req.alias, labels: {}, name: req.name, tags: [req.id] };
  }
}

class MyE2eHttpSecurityMatrix implements E2eHttpSecurityMatrix {
  async inherited_security(req: {
    resource_id: string;
    trace_id: string;
  }): Promise<string> {
    return `${req.resource_id}:${req.trace_id}`;
  }
  async bearer_or_cookie_security(req: {
    action: string;
    note?: string;
  }): Promise<string> {
    return `${req.action}:${formatOpt(req.note)}`;
  }
  async alternative_security(req: {
    resource_id: string;
    locale?: string;
  }): Promise<string> {
    return `${req.resource_id}:${formatOpt(req.locale)}`;
  }
  async oauth_security(req: {
    keyword: string;
    page?: number;
  }): Promise<string> {
    return `${req.keyword}:${formatOptInt(req.page)}`;
  }
  async public_ping(): Promise<string> {
    return 'pong';
  }
}

const typeServerState = {
  type_attr1: 'attr1',
  type_attr2: ['attr2'],
};

const attributeState = {
  attr1: 'attr1',
  attr2: ['attr2'],
  attr4: 'V1',
  attr61: { data: 1, tag: 'V1' },
};

let hostState = 'localhost';

async function readBodyString(req: any): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString('utf8');
}

const handlers = [
  createE2ePathSeverHandler(new MyE2ePathSever()),
  createE2eHttpRouteAndBodyHandler(new MyE2eHttpRouteAndBody()),
  createE2eHttpSecurityHandler(new MyE2eHttpSecurity()),
  createE2eTypeServerHandler(new MyE2eTypeServer()),
  createE2eAttributeHandler(new MyE2eAttribute()),
  createE2eHttpFormHandler(new MyE2eHttpForm()),
  createE2eHttpScopeMatrixHandler(new MyE2eHttpScopeMatrix()),
  createE2eHttpDefaultsMatrixHandler(new MyE2eHttpDefaultsMatrix()),
  createE2eHttpSecurityMatrixHandler(new MyE2eHttpSecurityMatrix()),
];

const port = process.env.PORT ? parseInt(process.env.PORT, 10) : 8080;
const server = createServer(async (req, res) => {
  try {
    let reqUrl = req.url || '';
    if (reqUrl.startsWith('/r/')) {
      reqUrl = `/v2/resources/${reqUrl.slice(3)}`;
    } else if (reqUrl.startsWith('/resources/')) {
      reqUrl = `/v2/resources/${reqUrl.slice(11)}`;
    }
    const protocol = req.headers['x-forwarded-proto'] || 'http';
    const hostHeader = req.headers.host || 'localhost';
    const url = new URL(reqUrl, `${protocol}://${hostHeader}`);

    const chunks: Buffer[] = [];
    for await (const chunk of req) {
      chunks.push(chunk);
    }
    const body = chunks.length > 0 ? Buffer.concat(chunks) : undefined;

    const requestHeaders = new Headers();
    for (const [key, value] of Object.entries(req.headers)) {
      if (Array.isArray(value)) {
        for (const val of value) {
          requestHeaders.append(key, val);
        }
      } else if (value !== undefined) {
        requestHeaders.set(key, value as string);
      }
    }

    const request = new Request(url.toString(), {
      body: req.method !== 'GET' && req.method !== 'HEAD' ? body : undefined,
      headers: requestHeaders,
      method: req.method,
    });

    let response: Response | null = null;
    for (const handler of handlers) {
      const resp = await handler(request);
      if (resp.status !== 404) {
        response = resp;
        break;
      }
    }

    if (!response) {
      response = new Response(JSON.stringify({ code: 404, msg: 'not found' }), {
        headers: { 'Content-Type': 'application/json' },
        status: 404,
      });
    }

    res.statusCode = response.status;
    response.headers.forEach((val, key) => {
      res.setHeader(key, val);
    });

    if (response.body) {
      if (typeof (response.body as any).getReader === 'function') {
        const reader = response.body.getReader();
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          res.write(value);
        }
      } else {
        const buffer = Buffer.from(await response.arrayBuffer());
        res.write(buffer);
      }
    }
    res.end();
  } catch (err) {
    console.error('Handler error:', err);
    res.statusCode = 500;
    res.end('Internal Server Error');
  }
});

server.listen(port, '127.0.0.1', () => {
  console.log(`TS server starting on port ${port}`);
});
