import { type NextRouteContext, XidlServerError } from 'xidl-typescript-server';
import { createNextRoute } from 'xidl-typescript-server/next';
import { UserServiceOperations } from '../../complex_rest.server';
import { userService } from '../../service';

const getUser = createNextRoute(UserServiceOperations.get_user, userService);

export async function GET(
  request: Request,
  context: NextRouteContext,
): Promise<Response> {
  try {
    return await getUser(request, context);
  } catch (error) {
    if (error instanceof XidlServerError) {
      return Response.json(
        { code: error.code, msg: error.msg },
        {
          headers: error.headers,
          status: error.code,
        },
      );
    }
    throw error;
  }
}
