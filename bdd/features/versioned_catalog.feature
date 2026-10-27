@bdd_rest-versioning
Feature: Media filters run on Head facts before historical catalog projection

  The Shopify-inspired catalog adds rich media while its old contract keeps
  images only. Both versions filter the same Head catalog, so a video-only
  product remains a match even when its old response has an empty media list.
  Both SDK versions use a shared named filter list without changing its wire
  shape, including empty, repeated and unrecognized media filters.
  Optional response cookies carry a separately submitted single-media preference:
  missing leaves it unchanged, empty explicitly selects a neutral value, and
  known media stores that value. A cookie-only update follows the same rules.

  Scenario Outline: repeated filters preserve business meaning across versions
    Given the versioned "catalog" application in "<mode>" mode
    When I generate independently selected SDKs and start the versioned application
    Then the shared versioned Hurl journey succeeds
    And every versioned Hurl response matches its generated OpenAPI contract
    And independent Rust and TypeScript SDKs use the same versioned endpoint

    Examples:
      | mode  |
      | chain |
      | route |
