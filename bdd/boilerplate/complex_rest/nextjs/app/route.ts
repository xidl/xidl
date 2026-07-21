import { createNextRoute } from 'xidl-typescript-server/next';
import { UserServiceOperations } from '../complex_rest.server';
import { userService } from '../service';

export const GET = createNextRoute(
  UserServiceOperations.list_users,
  userService,
);

export const POST = createNextRoute(
  UserServiceOperations.create_user,
  userService,
);
