import { createNextRoute } from 'xidl-typescript-server/next';
import { UserServiceOperations } from '../../complex_rest.server';
import { userService } from '../../service';

export const GET = createNextRoute(UserServiceOperations.get_user, userService);
