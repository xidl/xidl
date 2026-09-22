package xidlgohttp

import (
	"context"
	"errors"
	"io"
	"net"
	"net/http"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/gorilla/websocket"
)

const defaultWSWriteWait = 10 * time.Second

// WSBidiStream is a full-duplex typed WebSocket session (RFC 6455).
// One application message equals one WebSocket Text frame (JSON codec).
// TRead is the decoded inbound application message type.
// TWrite is the outbound application message type.
type WSBidiStream[TRead any, TWrite any] struct {
	conn *websocket.Conn
}

// NewWSBidiServer wraps an upgraded connection as a server-side session.
func NewWSBidiServer[TIn any, TOut any](conn *websocket.Conn) *WSBidiStream[TIn, TOut] {
	return &WSBidiStream[TIn, TOut]{conn: conn}
}

// NewWSBidiClient wraps a dialed connection as a client-side session.
// TIn is what the client writes, TOut is what the client reads.
func NewWSBidiClient[TIn any, TOut any](conn *websocket.Conn) *WSBidiStream[TOut, TIn] {
	return &WSBidiStream[TOut, TIn]{conn: conn}
}

// Read returns the next inbound application message.
// It returns io.EOF when the peer closes the stream cleanly.
func (s *WSBidiStream[TRead, TWrite]) Read() (TRead, error) {
	var zero TRead
	for {
		msgType, data, err := s.conn.ReadMessage()
		if err != nil {
			if websocket.IsCloseError(err, websocket.CloseNormalClosure, websocket.CloseGoingAway) ||
				errors.Is(err, io.EOF) || errors.Is(err, net.ErrClosed) ||
				websocket.IsUnexpectedCloseError(err) {
				return zero, io.EOF
			}
			return zero, err
		}
		switch msgType {
		case websocket.TextMessage, websocket.BinaryMessage:
			var value TRead
			if err := MustCodecForMime("application/json").Decode(strings.NewReader(string(data)), &value); err != nil {
				return zero, &HttpError{
					Status:  http.StatusBadRequest,
					Code:    http.StatusBadRequest,
					Message: "invalid ws payload: " + err.Error(),
				}
			}
			return value, nil
		case websocket.PingMessage:
			_ = s.conn.WriteControl(websocket.PongMessage, data, time.Now().Add(defaultWSWriteWait))
		case websocket.PongMessage, websocket.CloseMessage:
			if msgType == websocket.CloseMessage {
				return zero, io.EOF
			}
		default:
		}
	}
}

// Write sends one outbound application message as a WebSocket Text frame.
func (s *WSBidiStream[TRead, TWrite]) Write(value TWrite) error {
	var buf strings.Builder
	if err := MustCodecForMime("application/json").Encode(&buf, value); err != nil {
		return err
	}
	return s.conn.WriteMessage(websocket.TextMessage, []byte(buf.String()))
}

// Close sends a normal Close frame and releases the connection.
func (s *WSBidiStream[TRead, TWrite]) Close() error {
	msg := websocket.FormatCloseMessage(websocket.CloseNormalClosure, "")
	_ = s.conn.WriteControl(websocket.CloseMessage, msg, time.Now().Add(defaultWSWriteWait))
	return s.conn.Close()
}

// UpgradeWebSocket upgrades a gin request to RFC 6455 WebSocket, optionally
// requiring a single subprotocol when subprotocols has exactly one entry.
func UpgradeWebSocket(c *gin.Context, subprotocols []string) (*websocket.Conn, error) {
	if len(subprotocols) == 1 {
		offered := websocket.Subprotocols(c.Request)
		if len(offered) > 0 && !containsString(offered, subprotocols[0]) {
			return nil, &HttpError{
				Status:  http.StatusBadRequest,
				Code:    http.StatusBadRequest,
				Message: "missing required Sec-WebSocket-Protocol",
			}
		}
	}
	upgrader := websocket.Upgrader{
		CheckOrigin:  func(r *http.Request) bool { return true },
		Subprotocols: subprotocols,
	}
	return upgrader.Upgrade(c.Writer, c.Request, nil)
}

// DialWebSocket opens a client WebSocket connection to endpoint.
func DialWebSocket(ctx context.Context, endpoint string, subprotocols []string) (*websocket.Conn, error) {
	dialer := websocket.Dialer{
		Subprotocols: subprotocols,
		Proxy:        http.ProxyFromEnvironment,
	}
	header := http.Header{}
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	conn, _, err := dialer.DialContext(ctx, endpoint, header)
	if err != nil {
		return nil, err
	}
	return conn, nil
}

func containsString(values []string, want string) bool {
	for _, v := range values {
		if strings.EqualFold(v, want) {
			return true
		}
	}
	return false
}
