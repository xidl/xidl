@bdd_rest-versioning
Feature: A released messaging API keeps five explicit contracts

  Scenario: releases generate deterministic artifacts and detect historical edits
    Given the five-version Relay messaging corpus
    When I generate and select every messaging release through the CLI
    Then each selected document matches its complete history byte for byte
    And canonical diffs preserve the messaging lifecycle boundaries
    And a historical edit changes only the releases containing that declaration

  Scenario: invalid histories and lint mistakes fail through their own commands
    Given the five-version Relay messaging corpus
    Then every negative messaging input has its documented CLI outcome
    And unknown release selections fail without producing artifacts
