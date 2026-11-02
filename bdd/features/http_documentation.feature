@bdd_rest-versioning-core
Feature: Publish one versioned HTTP reference project
  Maintainers publish several API roots as one Starlight project, keep shared
  contracts coherent, and retain the last publication when a release is invalid.

  Scenario: Compile a multi-root manifest into a usable version comparison website
    Given an isolated multi-root HTTP documentation project
    When I publish the manifest, its directory, and the reversed explicit roots
    Then all publication bytes and shared versioned contracts agree
    When I build and browse the production Starlight project
    Then comparison navigation, search, copying, mobile, and no-script reading work

  Scenario: Narrow and restore a publication without deleting user-owned files
    Given an isolated multi-root HTTP documentation project
    When I publish all roots, then only billing v2, twice
    Then stale references, comparisons, and search entries disappear while user files survive
    When I restore the full manifest publication
    Then the original generated publication is restored byte for byte

  Scenario: Reject invalid releases without damaging the last publication and recover safely
    Given an isolated multi-root HTTP documentation project
    When I attempt invalid release notes, exclusions, settings, and example requests
    Then each failure preserves the last complete publication
    When I publish author HTML and code examples after correcting the release
    Then author markup stays literal and code examples remain copyable

  Scenario: Explore version-specific contracts and send real HTTP requests from Scalar
    Given an isolated multi-root HTTP documentation project
    When I browse generated version playgrounds against a local HTTP service
    Then successful and missing records preserve each API's selected version
