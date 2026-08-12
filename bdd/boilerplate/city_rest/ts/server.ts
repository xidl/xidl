import { createServer } from 'node:http';
import { createRouter } from 'xidl-typescript-server';
import type {
  SmartCityRestApiDownloadAssetResponse,
  SmartCityRestApiGetDeviceStatusResponse,
  SmartCityRestApiGetProfileResponse,
  SmartCityRestApiGetStopEtaResponse,
  SmartCityRestApiReserveLotResponse,
  SmartCityRestApiUpdateProfileResponse,
} from './city_rest.iface.js';
import {
  type SmartCityRestApi,
  SmartCityRestApiOperations,
} from './city_rest.server.js';

class MySmartCityRestService implements SmartCityRestApi {
  private maintenanceMode = false;

  async get_stop_eta(
    stop_id: string,
    line: string,
    locale: string,
  ): Promise<SmartCityRestApiGetStopEtaResponse> {
    return {
      destination: 'Central Station',
      eta_seconds: 240,
      return: stop_id,
    };
  }

  async list_nearby_stops(stop_id: string): Promise<string[]> {
    return [`${stop_id}-A`, `${stop_id}-B`];
  }

  async download_asset(
    asset_path: string,
    version: string,
  ): Promise<SmartCityRestApiDownloadAssetResponse> {
    return {
      content_type: 'text/plain',
      etag: 'etag-demo',
      return: Array.from(new TextEncoder().encode(`asset:${asset_path}`)),
    };
  }

  async probe_lot(lot_id: string, trace_id: string): Promise<void> {}

  async reserve_lot(
    lot_id: string,
    plate_number: string,
    minutes: number,
  ): Promise<SmartCityRestApiReserveLotResponse> {
    return {
      expires_at: '2026-03-08T10:00:00Z',
      reservation_state: 'CONFIRMED',
      return: `resv-${lot_id}`,
    };
  }

  async cancel_reservation(
    lot_id: string,
    reservation_id: string,
  ): Promise<void> {}

  async get_profile(
    citizen_id: string,
  ): Promise<SmartCityRestApiGetProfileResponse> {
    return {
      display_name: 'Taylor',
      language: 'en-US',
      phone_number: '+1-555-0101',
      return: citizen_id,
    };
  }

  async update_profile(
    citizen_id: string,
    display_name: string,
    phone_number: string,
  ): Promise<SmartCityRestApiUpdateProfileResponse> {
    return {
      audit_id: 'audit-20260307-001',
    };
  }

  async get_device_status(
    device_id: string,
    trace_id: string,
    session_id: string,
    locale: string,
  ): Promise<SmartCityRestApiGetDeviceStatusResponse> {
    return {
      return: `device:${device_id}`,
      session_echo: session_id,
      trace_echo: trace_id,
    };
  }

  async get_attribute_api_version(): Promise<string> {
    return 'v2.0.0';
  }

  async get_attribute_maintenance_mode(): Promise<boolean> {
    return this.maintenanceMode;
  }

  async set_attribute_maintenance_mode(
    maintenance_mode: boolean,
  ): Promise<void> {
    this.maintenanceMode = maintenance_mode;
  }
}

const service = new MySmartCityRestService();
const handler = createRouter(
  Object.values(SmartCityRestApiOperations),
  service,
);

const port = process.env.PORT ? parseInt(process.env.PORT, 10) : 8080;
const server = createServer(async (req, res) => {
  try {
    const protocol = req.headers['x-forwarded-proto'] || 'http';
    const host = req.headers.host || 'localhost';
    const url = new URL(req.url || '', `${protocol}://${host}`);

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

    const response = await handler(request);

    res.statusCode = response.status;
    response.headers.forEach((val, key) => {
      res.setHeader(key, val);
    });

    if (response.body) {
      const reader = response.body.getReader();
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        res.write(value);
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
