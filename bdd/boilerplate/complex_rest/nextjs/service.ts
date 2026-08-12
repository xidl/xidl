import { XidlServerError } from 'xidl-typescript-server';
import type { User } from './complex_rest';
import type { UserService } from './complex_rest.server';

class MyUserService implements UserService {
  private readonly users = new Map<number, User>();

  async get_user(id: number): Promise<User> {
    const user = this.users.get(id);
    if (!user) {
      throw new XidlServerError(404, 'Not Found');
    }
    return user;
  }

  async create_user(user: User): Promise<User> {
    this.users.set(user.id, user);
    return user;
  }

  async list_users(filter: string): Promise<User[]> {
    return [...this.users.values()].filter(
      user =>
        !filter || user.name.includes(filter) || user.roles.includes(filter),
    );
  }
}

const runtime = globalThis as typeof globalThis & {
  xidlUserService?: MyUserService;
};

runtime.xidlUserService ??= new MyUserService();

export const userService = runtime.xidlUserService;
