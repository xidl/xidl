import { XidlServerError } from 'xidl-typescript-server';
import type { User } from './complex_rest';
import type { UserService } from './complex_rest.server';

class MyUserService implements UserService {
  private readonly users = new Map<number, User>();

  async get_user(request: { id: number }): Promise<User> {
    const user = this.users.get(request.id);
    if (!user) {
      throw new XidlServerError('Not Found', 404);
    }
    return user;
  }

  async create_user(request: { user: User }): Promise<User> {
    this.users.set(request.user.id, request.user);
    return request.user;
  }

  async list_users(request: { filter: string }): Promise<User[]> {
    return [...this.users.values()].filter(
      user =>
        !request.filter ||
        user.name.includes(request.filter) ||
        user.roles.includes(request.filter),
    );
  }
}

const runtime = globalThis as typeof globalThis & {
  xidlUserService?: MyUserService;
};

runtime.xidlUserService ??= new MyUserService();

export const userService = runtime.xidlUserService;
