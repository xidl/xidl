import { createNextRoute } from 'xidl-typescript-server/next';
import { ReservedWordServiceOperations } from '../../reserved_word_params.server';
import { reservedWordService } from '../../service';

export const GET = createNextRoute(
  ReservedWordServiceOperations.search,
  reservedWordService,
);
