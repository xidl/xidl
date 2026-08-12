import { createServer } from 'node:http';
import { encode } from '@msgpack/msgpack';
import { createRouter } from 'xidl-typescript-server';
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
  type E2eAttribute,
  E2eAttributeOperations,
  type E2eHttpDefaultsMatrix,
  E2eHttpDefaultsMatrixOperations,
  type E2eHttpForm,
  E2eHttpFormOperations,
  type E2eHttpRouteAndBody,
  E2eHttpRouteAndBodyOperations,
  type E2eHttpScopeMatrix,
  E2eHttpScopeMatrixOperations,
  type E2eHttpSecurity,
  type E2eHttpSecurityMatrix,
  E2eHttpSecurityMatrixOperations,
  E2eHttpSecurityOperations,
  type E2ePathSever,
  E2ePathSeverOperations,
  type E2eTypeServer,
  E2eTypeServerOperations,
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
  async op_with_path(param1: string): Promise<string[]> {
    return [param1];
  }
  async op_with_query(param1: string, q: string): Promise<string[]> {
    return [param1, q];
  }
  async op_with_params(
    path_name: string,
    q: string[],
    b: number[],
    a: Record<string, any>,
  ): Promise<string[]> {
    const res = [path_name, ...q];
    res.push(JSON.stringify(Array.from(b)));
    res.push(JSON.stringify(a));
    return res;
  }
  async op_with_query2(all: string, word: string, q: string): Promise<string> {
    return `${all}:${word}:${q}`;
  }
}

class MyE2eHttpRouteAndBody implements E2eHttpRouteAndBody {
  async get_resource(
    resource_id: string,
    locale: string | undefined,
    trace_id: string,
  ): Promise<string> {
    return `id:${resource_id},lang:${formatOpt(locale)},trace:${trace_id}`;
  }
  async get_file(
    file_path: string,
    download: boolean,
    version: string | undefined,
  ): Promise<string> {
    let filePath = file_path;
    if (filePath.startsWith('/')) {
      filePath = filePath.slice(1);
    }
    return `file:${filePath},download:${download},version:${formatOpt(version)}`;
  }
  async create_resource(resource_body: StructHttpBody): Promise<StructHttpBody> {
    return resource_body;
  }
  async replace_resource(
    resource_id: string,
    etag: string,
    payload: StructHttpBody,
  ): Promise<void> {}
  async patch_resource(
    resource_id: string,
    dry_run: boolean,
    session_id: string,
    changes: Record<string, any>,
  ): Promise<Record<string, any>> {
    return changes;
  }
  async delete_resource(
    resource_id: string,
    force: boolean | undefined,
  ): Promise<void> {}
  async probe_resource(
    resource_id: string,
    if_none_match: string,
  ): Promise<void> {}
  async resource_options(resource_id: string): Promise<void> {}
  async get_msgpack_resource(
    resource_id: string,
  ): Promise<E2EHttpRouteAndBodyGetMsgpackResourceResponse> {
    return {
      return: { labels: {}, name: 'msgpack', tags: [] },
      revision: 1,
    };
  }
  async dedup_resource(id: string, x_trace_id: string): Promise<string> {
    return `${id}:${x_trace_id}`;
  }
  async preview_resource(resource: StructHttpBody): Promise<StructHttpBody> {
    return resource;
  }
}

class MyE2eHttpSecurity implements E2eHttpSecurity {
  async get_secure_user(
    user_id: string,
    locale: string | undefined,
    trace_id: string,
  ): Promise<string> {
    return `user:${user_id},lang:${formatOpt(locale)},trace:${trace_id}`;
  }
  async search_secure_user(
    keyword: string,
    page: number | undefined,
  ): Promise<string> {
    return `keyword:${keyword},page:${formatOptInt(page)}`;
  }
  async healthz(): Promise<string> {
    return 'ok';
  }
}

class MyE2eTypeServer implements E2eTypeServer {
  async get_attribute_type_attr1(): Promise<string> {
    return typeServerState.type_attr1;
  }
  async set_attribute_type_attr1(type_attr_1: string): Promise<void> {
    typeServerState.type_attr1 = type_attr_1;
  }
  async get_attribute_type_attr2(): Promise<string[]> {
    return typeServerState.type_attr2;
  }
  async simple_op(): Promise<void> {}
  async simple_op_with_return1(): Promise<string> {
    return 'simple_op_with_return1';
  }
  async simple_op_with_return2(): Promise<EnumEmpty> {
    return {} as EnumEmpty;
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
  async return_with_sequence2(): Promise<EnumEmpty[]> {
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
  async parameter_op(a: string): Promise<void> {}
  async parameter_op2(a: string): Promise<void> {}
  async parameter_op3(
    a: string,
    c: any[],
  ): Promise<E2ETypeServerParameterOp3Response> {
    return { b: 3, c: [] };
  }
  async parameter_op4(c: any[]): Promise<E2ETypeServerParameterOp4Response> {
    return { a: 'op4', b: 4, c: [] };
  }
  async parameter_op5(c: any[]): Promise<E2ETypeServerParameterOp5Response> {
    return { a: 'op5', b: 5, c: [], return: ['op5'] };
  }
  async parameter_op6(c: any[]): Promise<E2ETypeServerParameterOp6Response> {
    return { a: 'op6', b: 6, c: [], return: {} };
  }
}

class MyE2eAttribute implements E2eAttribute {
  async get_attribute_attr1(): Promise<string> {
    return attributeState.attr1;
  }
  async set_attribute_attr1(attr_1: string): Promise<void> {
    attributeState.attr1 = attr_1;
  }
  async get_attribute_attr2(): Promise<string[]> {
    return attributeState.attr2;
  }
  async get_attribute_attr3(): Promise<EnumEmpty> {
    return {} as EnumEmpty;
  }
  async set_attribute_attr3(attr_3: EnumEmpty): Promise<void> {}
  async get_attribute_attr4(): Promise<EnumSimple1> {
    return attributeState.attr4 as EnumSimple1;
  }
  async set_attribute_attr4(attr_4: EnumSimple1): Promise<void> {
    attributeState.attr4 = attr_4;
  }
  async get_attribute_attr5(): Promise<StructEmpty> {
    return {};
  }
  async set_attribute_attr5(attr_5: StructEmpty): Promise<void> {}
  async get_attribute_attr6(): Promise<StructSimple> {
    return {
      member1: {} as EnumEmpty,
      member2: 'V1' as EnumSimple1,
      member3: {},
    };
  }
  async set_attribute_attr6(attr_6: StructSimple): Promise<void> {}
  async get_attribute_attr61(): Promise<UnionSimple> {
    return attributeState.attr61 as unknown as UnionSimple;
  }
  async set_attribute_attr61(attr_61: UnionSimple): Promise<void> {
    attributeState.attr61 = attr_61 as any;
  }
  async get_attribute_attr7(): Promise<string[]> {
    return [];
  }
  async set_attribute_attr7(attr_7: string[]): Promise<void> {}
  async get_attribute_attr8(): Promise<EnumEmpty[]> {
    return [];
  }
  async set_attribute_attr8(attr_8: EnumEmpty[]): Promise<void> {}
  async get_attribute_attr9(): Promise<EnumSimple1[]> {
    return [];
  }
  async set_attribute_attr9(attr_9: EnumSimple1[]): Promise<void> {}
  async get_attribute_attr10(): Promise<StructEmpty[]> {
    return [];
  }
  async set_attribute_attr10(attr_10: StructEmpty[]): Promise<void> {}
  async get_attribute_attr11(): Promise<StructSimple[]> {
    return [];
  }
  async set_attribute_attr11(attr_11: StructSimple[]): Promise<void> {}
  async get_attribute_attr12(): Promise<Record<string, number>> {
    return {};
  }
  async set_attribute_attr12(attr_12: Record<string, number>): Promise<void> {}
  async get_attribute_attr13(): Promise<any> {
    return null;
  }
  async set_attribute_attr13(attr_13: any): Promise<void> {}
  async get_attribute_attr14(): Promise<any[]> {
    return [];
  }
  async set_attribute_attr14(attr_14: any[]): Promise<void> {}
  async get_attribute_attr15(): Promise<Record<string, any>> {
    return {};
  }
  async set_attribute_attr15(attr_15: Record<string, any>): Promise<void> {}
  async get_attribute_attr16(): Promise<string> {
    return 'attr16';
  }
}

class MyE2eHttpForm implements E2eHttpForm {
  async submit_profile(
    name: string,
    age: number | undefined,
  ): Promise<E2EHttpFormSubmitProfileResponse> {
    return {
      normalized_name: name.toUpperCase(),
      return: `name:${name},age:${formatOptInt(age)}`,
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
  async default_scope(request_body: StructHttpBody): Promise<string> {
    return request_body.name;
  }
  async override_consumes_only(
    name: string,
    age: number | undefined,
  ): Promise<E2EHttpScopeMatrixOverrideConsumesOnlyResponse> {
    return {
      normalized_name: name.toUpperCase(),
      return: `name:${name},age:${formatOptInt(age)}`,
    };
  }
  async override_produces_only(
    resource_id: string,
  ): Promise<E2EHttpScopeMatrixOverrideProducesOnlyResponse> {
    return {
      return: { labels: {}, name: resource_id, tags: [] },
      revision: 1,
    };
  }
  async override_both_media(
    name: string,
    age: number | undefined,
  ): Promise<E2EHttpScopeMatrixOverrideBothMediaResponse> {
    return {
      normalized_name: 'OVERRIDDEN',
      return: {
        labels: {},
        name: name,
        tags: [`age:${formatOptInt(age)}`],
      },
    };
  }
  async deprecated_plain(resource_id: string): Promise<string> {
    return resource_id;
  }
  async deprecated_since_only(resource_id: string): Promise<string> {
    return resource_id;
  }
  async deprecated_window(resource_id: string): Promise<string> {
    return resource_id;
  }
}

class MyE2eHttpDefaultsMatrix implements E2eHttpDefaultsMatrix {
  async delete_resource_default_query(
    id: string,
    revision: number,
  ): Promise<string> {
    return `${id}:${revision}`;
  }
  async probe_resource_default_query(
    id: string,
    revision: number,
  ): Promise<void> {}
  async resource_options_default_query(
    id: string,
    revision: number,
  ): Promise<void> {}
  async replace_resource_default_body(
    id: string,
    name: string,
    alias: string | undefined,
  ): Promise<StructHttpBody> {
    return { alias: alias, labels: {}, name: name, tags: [id] };
  }
  async patch_resource_default_body(
    id: string,
    name: string,
    alias: string | undefined,
  ): Promise<StructHttpBody> {
    return { alias: alias, labels: {}, name: name, tags: [id] };
  }
}

class MyE2eHttpSecurityMatrix implements E2eHttpSecurityMatrix {
  async inherited_security(
    resource_id: string,
    trace_id: string,
  ): Promise<string> {
    return `${resource_id}:${trace_id}`;
  }
  async bearer_or_cookie_security(
    action: string,
    note: string | undefined,
  ): Promise<string> {
    return `${action}:${formatOpt(note)}`;
  }
  async alternative_security(
    resource_id: string,
    locale: string | undefined,
  ): Promise<string> {
    return `${resource_id}:${formatOpt(locale)}`;
  }
  async oauth_security(
    keyword: string,
    page: number | undefined,
  ): Promise<string> {
    return `${keyword}:${formatOptInt(page)}`;
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

const hostState = 'localhost';

const msgpackOptions = {
  codecs: {
    'application/msgpack': { encode },
  },
};

async function readBodyString(req: any): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString('utf8');
}

const handlers = [
  createRouter(Object.values(E2ePathSeverOperations), new MyE2ePathSever()),
  createRouter(
    Object.values(E2eHttpRouteAndBodyOperations),
    new MyE2eHttpRouteAndBody(),
    msgpackOptions,
  ),
  createRouter(
    Object.values(E2eHttpSecurityOperations),
    new MyE2eHttpSecurity(),
  ),
  createRouter(Object.values(E2eTypeServerOperations), new MyE2eTypeServer()),
  createRouter(Object.values(E2eAttributeOperations), new MyE2eAttribute()),
  createRouter(Object.values(E2eHttpFormOperations), new MyE2eHttpForm()),
  createRouter(
    Object.values(E2eHttpScopeMatrixOperations),
    new MyE2eHttpScopeMatrix(),
    msgpackOptions,
  ),
  createRouter(
    Object.values(E2eHttpDefaultsMatrixOperations),
    new MyE2eHttpDefaultsMatrix(),
  ),
  createRouter(
    Object.values(E2eHttpSecurityMatrixOperations),
    new MyE2eHttpSecurityMatrix(),
  ),
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
