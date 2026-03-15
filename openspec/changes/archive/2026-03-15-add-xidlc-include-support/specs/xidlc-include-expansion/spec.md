## ADDED Requirements

### Requirement: HIR SHALL expand include files in source order
When HIR construction encounters `#include`, it SHALL parse the referenced file and merge the included definitions at the exact position where the directive appears in the containing file.

#### Scenario: Include content is inserted between local definitions
- **WHEN** a source file declares a definition, then `#include`s another IDL file, then declares another definition
- **THEN** the resulting HIR places the included file's definitions between the two local definitions in the same relative order

#### Scenario: Nested includes preserve depth-first order
- **WHEN** an included file itself contains another `#include`
- **THEN** HIR expansion processes the nested file before continuing with later definitions in the parent file

### Requirement: Include paths SHALL resolve relative to the including file
The system SHALL resolve file-based include paths against the directory of the file that contains the `#include` directive, not the current working directory of the compiler process.

#### Scenario: Sibling fixture is loaded from the including file directory
- **WHEN** `dir/main.idl` contains `#include "shared/common.idl"`
- **THEN** the compiler reads `dir/shared/common.idl` even if the process was started from a different working directory

### Requirement: Invalid include targets SHALL fail HIR construction
The system SHALL reject include expansion when the target file cannot be read, cannot be parsed into a valid typed AST, or would cause recursive include expansion.

#### Scenario: Missing include file stops compilation
- **WHEN** a source file references an include path that does not exist
- **THEN** HIR construction fails with a diagnostic identifying the unresolved include path

#### Scenario: Included file with invalid AST stops compilation
- **WHEN** an included file contains invalid IDL and parser_text cannot produce a typed AST
- **THEN** HIR construction fails instead of silently dropping the include

#### Scenario: Cyclic include graph is rejected
- **WHEN** file `a.idl` includes `b.idl` and `b.idl` includes `a.idl`
- **THEN** HIR construction fails with a diagnostic that identifies the recursive include chain
