@bdd_rest-versioning-core
Feature: Integer constants retain IDL values in generated Rust
  Target compilers calculate ordinary constants; fixed-array sizes are resolved by xidl.
  Independent imports use their declared widths, and local module names cannot
  change the value of an explicitly rooted dependency reference.

  Scenario: Split constant declarations compile and preserve their values
    Given a REST IDL file "bdd/features/data/integer_constants/api.idl"
    When I generate the split integer constant Rust library
    Then the compiled constant library preserves masks and imported values

  Scenario Outline: Target compilers reject invalid arithmetic instead of wrapping
    Given a REST IDL file "bdd/features/data/integer_constants/api.idl"
    When I generate the integer expression "<declaration>"
    Then Rust and Go reject the generated integer contract

    Examples:
      | declaration                                                                                  |
      | const unsigned long BAD = (core::MASK32 + 1) - 1;                                              |
      | const unsigned long BAD = 0 >> 64;                                                            |
      | const unsigned long BAD = 1 << 64;                                                            |
      | const unsigned long BAD = 1 << -1;                                                            |
      | const unsigned long BAD = core::ALL64 & 0;                                                     |
      | const unsigned long BAD = -2 + 1;                                                             |
      | const long BAD = ~0;                                                                         |
      | const short BAD = 65536;                                                                     |
      | const long BAD = 1 / 0;                                                                      |
      | const long BAD = 1 % 0;                                                                      |
      | const unsigned long long BAD = core::ALL64 * core::ALL64;                                      |
