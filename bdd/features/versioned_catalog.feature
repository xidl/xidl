@bdd_rest-versioning
Feature: Media filters run on Head facts before historical catalog projection

  The Shopify-inspired catalog adds rich media while its old contract keeps
  images only. Both versions filter the same Head catalog, so a video-only
  product remains a match even when its old response has an empty media list.
  Both SDK versions use a shared named filter list without changing its wire
  shape, including empty, repeated and unrecognized media filters.

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
