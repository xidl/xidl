@bdd_rest-versioning
Feature: Independently versioned SDKs share one authoritative billing ledger

  This reduced contract is inspired by Stripe's partial invoice payments change.
  The application selects the version; generated projections never inspect HTTP.
  Chain dispatch and generated version routers share the same Head service.

  Scenario Outline: partial payments preserve contracts and committed replays
    Given the versioned "billing" application in "<mode>" mode
    When I generate independently selected SDKs and start the versioned application
    Then the shared versioned Hurl journey succeeds
    And every versioned Hurl response matches its generated OpenAPI contract
    And independent Rust and TypeScript SDKs use the same versioned endpoint

    Examples:
      | mode  |
      | chain |
      | route |
