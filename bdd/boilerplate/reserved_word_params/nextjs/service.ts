import type { ReservedWordService } from './reserved_word_params.server';

class MyReservedWordService implements ReservedWordService {
  async get_monitor(request: { id: string; type: string }): Promise<string> {
    return `monitor:${request.id}:${request.type}`;
  }

  async search(request: { type: string }): Promise<string> {
    return `search:${request.type}`;
  }
}

const runtime = globalThis as typeof globalThis & {
  xidlReservedWordService?: MyReservedWordService;
};

runtime.xidlReservedWordService ??= new MyReservedWordService();

export const reservedWordService = runtime.xidlReservedWordService;
