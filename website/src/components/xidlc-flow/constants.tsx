export type CodeLanguage = 'idl' | 'rust' | 'typescript';

export interface CodeSample {
  language: CodeLanguage;
  lines: string[];
}

export const XIDL: CodeSample = {
  language: 'idl',
  lines: [
    'module shop {',
    '  struct User {',
    '    uint64 id,',
    '    string name,',
    '  }',
    '',
    '  interface UserService {',
    '    User create_user(name: string);',
    '  }',
    '}',
  ],
};

export const WEB: CodeSample = {
  language: 'typescript',
  lines: [
    'import { createUser, type User } from "generated";',
    '',
    'export async function handleCreateUser(',
    '  name: string,',
    '): Promise<User> {',
    '  return await createUser(name);',
    '}',
  ],
};

export const RUST: CodeSample = {
  language: 'rust',
  lines: [
    'use generated::user::{self, User};',
    '',
    'struct Service;',
    '',
    'impl user::UserService for Service {',
    '    async fn create_user(&self, name: String)',
    '        -> Result<User> {',
    '        Ok(User { id: 1, name })',
    '    }',
    '}',
  ],
};

export const KEYWORDS: Record<CodeLanguage, string> = {
  idl: 'module|struct|interface',
  rust: 'use|struct|impl|for|async|fn|self|Ok',
  typescript: 'import|from|type|export|async|function|await|return|as',
};

export const CAPTIONS = [
  'Write the xidl interface definition',
  'Compile it with xidlc',
  'Import generated code, write business logic',
  'Run the website, call the Rust service',
];
