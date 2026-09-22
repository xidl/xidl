Feature: WebSocket Upgrade Generation and Communication
  As a developer
  I want @upgrade(protocol = "websocket") to produce a real RFC 6455 WebSocket
  So that browsers and standard WS clients can talk to generated services

  Scenario: WebSocket control channel round-trip with subprotocol
    Given a REST IDL file "bdd/features/data/websocket_upgrade.idl"
    When I generate rust code for the IDL
    Then the generated rust code should be valid
    And I can run the generated rust server using boilerplate
    And the websocket client can send opcode "ping" and payload "hello" and receive event "ping:ack" and data "hello"
    And the websocket handshake negotiates subprotocol "fastnet.v1"
    And the websocket connection answers ping with pong

  Scenario: Reject ws protocol alias
    Given a REST IDL file "bdd/features/data/websocket_bad_protocol.idl"
    When I generate rust code for the IDL and expect failure containing "websocket"

  Scenario: Reject websocket-only params on raw upgrade
    Given a REST IDL file "bdd/features/data/websocket_raw_with_ws_params.idl"
    When I generate rust code for the IDL and expect failure containing "only valid with protocol"
