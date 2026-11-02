const primitiveTypes =
  'void|short|long|float|double|boolean|char|wchar|string|wstring|octet|fixed|any|object|valuebase|int8|uint8|int16|uint16|int32|uint32|int64|uint64';

const identifier = '[A-Za-z_][A-Za-z0-9_]*';

const idlLanguage = {
  aliases: ['xidl'],
  displayName: 'IDL',
  fileTypes: ['idl'],
  name: 'idl',
  patterns: [
    { include: '#comments' },
    { include: '#pragma_directive' },
    { include: '#annotations' },
    { include: '#enum_block' },
    { include: '#type_declaration' },
    { include: '#member_declaration' },
    { include: '#keywords' },
    { include: '#types' },
    { include: '#constants' },
    { include: '#numbers' },
    { include: '#strings' },
    { include: '#functions' },
    { include: '#operators' },
    { include: '#punctuation' },
  ],
  repository: {
    annotations: {
      patterns: [
        {
          begin: `(@${identifier})(\\()`,
          beginCaptures: {
            1: { name: 'entity.name.function.annotation.idl' },
            2: { name: 'punctuation.section.group.begin.idl' },
          },
          end: '(\\))',
          endCaptures: {
            1: { name: 'punctuation.section.group.end.idl' },
          },
          name: 'meta.annotation.idl',
          patterns: [
            {
              match: `\\b(${identifier})\\b(?=\\s*=)`,
              name: 'variable.parameter.idl',
            },
            { include: '#strings' },
            { include: '#numbers' },
            { include: '#operators' },
            { include: '#punctuation' },
          ],
        },
        {
          match: `@${identifier}`,
          name: 'entity.name.function.annotation.idl',
        },
      ],
    },
    comments: {
      patterns: [
        {
          begin: '/\\*',
          end: '\\*/',
          name: 'comment.block.idl',
        },
        {
          captures: {
            1: { name: 'punctuation.definition.comment.idl' },
          },
          match: '(//).*$\\n?',
          name: 'comment.line.double-slash.idl',
        },
      ],
    },
    constants: {
      patterns: [
        {
          match: '\\b(?:TRUE|FALSE|true|false)\\b',
          name: 'constant.language.boolean.idl',
        },
      ],
    },
    enum_block: {
      begin: `\\b(enum|bitmask|bitset)\\b\\s+(${identifier})\\s*(\\{)`,
      beginCaptures: {
        1: { name: 'storage.type.enum.idl' },
        2: { name: 'entity.name.type.enum.idl' },
        3: { name: 'punctuation.section.block.begin.idl' },
      },
      end: '(\\})\\s*(;)?',
      endCaptures: {
        1: { name: 'punctuation.section.block.end.idl' },
        2: { name: 'punctuation.terminator.statement.idl' },
      },
      name: 'meta.enum.idl',
      patterns: [
        { include: '#comments' },
        { include: '#annotations' },
        {
          captures: {
            1: { name: 'constant.other.enum-member.idl' },
            2: { name: 'punctuation.separator.delimiter.idl' },
          },
          match: `\\b(${identifier})\\b\\s*(,)?`,
        },
      ],
    },
    functions: {
      patterns: [
        {
          captures: {
            1: { name: 'entity.name.function.idl' },
          },
          match: `\\b(${identifier})\\b(?=\\s*\\()`,
        },
      ],
    },
    keywords: {
      patterns: [
        {
          match:
            '\\b(?:inout|in|out|readonly|oneway|raises|context|switch|case|default)\\b',
          name: 'keyword.control.idl',
        },
        {
          match: '\\b(?:sequence|map|set)\\b',
          name: 'storage.type.generic.idl',
        },
      ],
    },
    member_declaration: {
      patterns: [
        {
          captures: {
            1: { name: 'support.type.primitive.idl' },
            2: { name: 'variable.other.member.idl' },
            3: { name: 'punctuation.terminator.statement.idl' },
          },
          match: `\\b(${primitiveTypes})\\b\\s+(${identifier})\\s*(;)`,
        },
      ],
    },
    numbers: {
      patterns: [
        {
          match: '\\b(?:0[xX][0-9A-Fa-f]+|\\d+(?:\\.\\d+)?)\\b',
          name: 'constant.numeric.idl',
        },
      ],
    },
    operators: {
      patterns: [
        {
          match: '=',
          name: 'keyword.operator.assignment.idl',
        },
      ],
    },
    pragma_directive: {
      begin: '^(\\s*)(#pragma)\\b',
      beginCaptures: {
        2: { name: 'keyword.control.directive.idl' },
      },
      end: '$',
      name: 'meta.preprocessor.pragma.idl',
      patterns: [
        {
          match: '\\b(?:xidlc|package|version)\\b',
          name: 'entity.name.tag.pragma.idl',
        },
        { include: '#strings' },
        { include: '#numbers' },
      ],
    },
    punctuation: {
      patterns: [
        {
          match: '[{}()\\[\\],;<>:]',
          name: 'punctuation.separator.idl',
        },
      ],
    },
    strings: {
      patterns: [
        {
          begin: '"',
          beginCaptures: {
            0: { name: 'punctuation.definition.string.begin.idl' },
          },
          end: '"',
          endCaptures: {
            0: { name: 'punctuation.definition.string.end.idl' },
          },
          name: 'string.quoted.double.idl',
        },
        {
          begin: "'",
          beginCaptures: {
            0: { name: 'punctuation.definition.string.begin.idl' },
          },
          end: "'",
          endCaptures: {
            0: { name: 'punctuation.definition.string.end.idl' },
          },
          name: 'string.quoted.single.idl',
        },
      ],
    },
    type_declaration: {
      patterns: [
        {
          captures: {
            1: { name: 'storage.type.namespace.idl' },
            2: { name: 'entity.name.namespace.idl' },
          },
          match: `\\b(module)\\b\\s+(${identifier})`,
        },
        {
          captures: {
            1: { name: 'storage.type.interface.idl' },
            2: { name: 'entity.name.type.interface.idl' },
          },
          match: `\\b(interface)\\b\\s+(${identifier})`,
        },
        {
          captures: {
            1: { name: 'storage.type.struct.idl' },
            2: { name: 'entity.name.type.struct.idl' },
          },
          match: `\\b(struct|union|exception)\\b\\s+(${identifier})`,
        },
        {
          captures: {
            1: { name: 'storage.type.typedef.idl' },
            2: { name: 'entity.name.type.typedef.idl' },
          },
          match: `\\b(typedef|const|attribute|native)\\b\\s+(${identifier})`,
        },
      ],
    },
    types: {
      patterns: [
        {
          match: `\\b(?:${primitiveTypes})\\b`,
          name: 'support.type.primitive.idl',
        },
      ],
    },
  },
  scopeName: 'source.idl',
};

export default idlLanguage;
