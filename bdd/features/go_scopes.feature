@bdd_rest-errors
Feature: Go modules retain IDL type identity
  Included and imported definitions keep their declaring scope through containers,
  inheritance and interface exports, even when another scope declares the same name.

  Scenario Outline: Split modules preserve type identity in models and HTTP
    Given a REST IDL file "bdd/features/data/go_scopes/api.idl"
    When I generate the scoped Go contract with the "<target>" target
    Then the compiled Go contract preserves module type identity

    Examples:
      | target  |
      | go      |
      | go-rest |
