package xidlgohttp

import (
	"context"
	"errors"
	"fmt"
	json "github.com/xidl/xidl/golang/xidl-go-codec"
	"io"
)

// StreamOutcome is set once when a structured stream terminates.
type StreamOutcome string

const (
	StreamCompleted   StreamOutcome = "Completed"
	StreamFailed      StreamOutcome = "Failed"
	StreamCancelled   StreamOutcome = "Cancelled"
	StreamInterrupted StreamOutcome = "Interrupted"
)

// StreamFailure reuses an operation's HTTP exception status and JSON body.
type StreamFailure struct {
	Status int             `json:"status" xjson:"status"`
	Body   json.RawMessage `json:"body" xjson:"body"`
}

// StreamInterruptedError distinguishes truncation from a complete frame.
type StreamInterruptedError struct{ Cause error }

func (e *StreamInterruptedError) Error() string {
	return fmt.Sprintf("stream interrupted: %v", e.Cause)
}
func (e *StreamInterruptedError) Unwrap() error { return e.Cause }

// StreamFrame carries an explicit data field, including zero, false and null.
type StreamFrame[T any] struct {
	Type  string         `json:"t" xjson:"t"`
	Seq   uint64         `json:"seq,omitempty" xjson:"seq,omitempty"`
	Data  T              `json:"data" xjson:"data"`
	Error *StreamFailure `json:"error,omitempty" xjson:"error,omitempty"`
}

type ServerStreamWriter[T any] interface {
	Write(T) error
	Close() error
}

func EncodeStreamError(err error) StreamFailure {
	status := 500
	var httpError *HttpError
	if errors.As(err, &httpError) {
		status = httpError.Status
	}
	body, marshalErr := json.Marshal(map[string]any{"code": status, "msg": err.Error()})
	if marshalErr != nil {
		body = []byte(`{"code":500,"msg":"stream error encoding failed"}`)
		status = 500
	}
	return StreamFailure{Status: status, Body: body}
}

func DecodeStreamError(failure StreamFailure) error {
	var body struct {
		Code    int    `json:"code"`
		Message string `json:"msg"`
	}
	if json.Unmarshal(failure.Body, &body) == nil && body.Message != "" {
		return &HttpError{Status: failure.Status, Code: body.Code, Message: body.Message}
	}
	return NewHttpError(failure.Status, "undeclared stream exception: "+string(failure.Body))
}

type ClientStreamReader[T any] struct {
	ctx    context.Context
	reader *ServerStreamReader[T]
}

func NewClientStreamReader[T any](ctx context.Context, body io.Reader) *ClientStreamReader[T] {
	closer, ok := body.(io.ReadCloser)
	if !ok {
		closer = io.NopCloser(body)
	}
	reader := NewNDJSONStreamReader[T](closer)
	reader.ctx = ctx
	return &ClientStreamReader[T]{ctx: ctx, reader: reader}
}
func (r *ClientStreamReader[T]) Read() (T, error) {
	if r.reader.Outcome() != "" {
		return r.reader.Read()
	}
	select {
	case <-r.ctx.Done():
		var zero T
		r.reader.Close()
		return zero, r.ctx.Err()
	default:
	}
	return r.reader.Read()
}
func (r *ClientStreamReader[T]) Outcome() StreamOutcome { return r.reader.Outcome() }
