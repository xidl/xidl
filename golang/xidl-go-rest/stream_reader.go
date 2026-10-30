package xidlgohttp

import (
	"bufio"
	"context"
	stdjson "encoding/json"
	"errors"
	json "github.com/xidl/xidl/golang/xidl-go-codec"
	"io"
	"strings"
	"sync"
)

type streamReader struct {
	body        io.ReadCloser
	outcome     StreamOutcome
	decodeError func(HTTPFailure) error
	ctx         context.Context
	closeOnce   sync.Once
	closeError  error
}

func (r *streamReader) Outcome() StreamOutcome { return r.outcome }
func (r *streamReader) release() error {
	r.closeOnce.Do(func() { r.closeError = r.body.Close() })
	return r.closeError
}
func (r *streamReader) finish(outcome StreamOutcome) {
	if r.outcome == "" {
		r.outcome = outcome
	}
	_ = r.release()
}
func (r *streamReader) Close() error {
	r.finish(StreamCancelled)
	return r.closeError
}
func (r *streamReader) interrupted(err error) error {
	if r.ctx != nil && r.ctx.Err() != nil {
		r.finish(StreamCancelled)
		return r.ctx.Err()
	}
	r.finish(StreamInterrupted)
	if err == io.EOF {
		err = io.ErrUnexpectedEOF
	}
	return &StreamInterruptedError{Cause: err}
}
func (r *streamReader) failure(raw []byte) error {
	var failure HTTPFailure
	// Preserve explicit JSON null separately from an absent error body.
	if err := stdjson.Unmarshal(raw, &failure); err != nil {
		return r.interrupted(err)
	}
	if failure.Status < 300 || failure.Status > 599 || failure.Body == nil {
		return r.interrupted(errors.New("invalid stream error frame"))
	}
	r.finish(StreamFailed)
	return r.decodeError(failure)
}

type ServerStreamReader[T any] struct {
	*streamReader
	dec *json.Decoder
}

func NewNDJSONStreamReader[T any](body io.ReadCloser) *ServerStreamReader[T] {
	return &ServerStreamReader[T]{streamReader: &streamReader{body: body, decodeError: DecodeHTTPError}, dec: json.NewDecoder(body)}
}
func (r *ServerStreamReader[T]) WithErrorDecoder(decode func(HTTPFailure) error) *ServerStreamReader[T] {
	r.decodeError = decode
	return r
}
func (r *ServerStreamReader[T]) Read() (T, error) {
	var zero T
	if r.outcome != "" {
		return zero, io.EOF
	}
	for {
		var raw map[string]json.RawMessage
		if err := r.dec.Decode(&raw); err != nil {
			return zero, r.interrupted(err)
		}
		var kind string
		if err := json.Unmarshal(raw["t"], &kind); err != nil {
			return zero, r.interrupted(err)
		}
		switch kind {
		case "heartbeat":
			continue
		case "complete":
			r.finish(StreamCompleted)
			return zero, io.EOF
		case "error":
			return zero, r.failure(raw["error"])
		case "next":
			data, ok := raw["data"]
			if !ok {
				return zero, r.interrupted(errors.New("NDJSON next frame requires data"))
			}
			var item T
			if err := json.Unmarshal(data, &item); err != nil {
				return zero, r.interrupted(err)
			}
			return item, nil
		default:
			return zero, r.interrupted(errors.New("unknown NDJSON frame"))
		}
	}
}

type SSEStreamReader[T any] struct {
	*streamReader
	lines *bufio.Reader
}

func NewSSEStreamReader[T any](body io.ReadCloser) *SSEStreamReader[T] {
	return &SSEStreamReader[T]{streamReader: &streamReader{body: body, decodeError: DecodeHTTPError}, lines: bufio.NewReader(body)}
}

// WithContext distinguishes explicit cancellation from a broken transport.
func (r *SSEStreamReader[T]) WithContext(ctx context.Context) *SSEStreamReader[T] {
	r.ctx = ctx
	return r
}
func (r *SSEStreamReader[T]) WithErrorDecoder(decode func(HTTPFailure) error) *SSEStreamReader[T] {
	r.decodeError = decode
	return r
}
func (r *SSEStreamReader[T]) Read() (T, error) {
	var zero T
	if r.outcome != "" {
		return zero, io.EOF
	}
	if r.ctx != nil && r.ctx.Err() != nil {
		return zero, r.interrupted(r.ctx.Err())
	}
	event := ""
	var data []string
	for {
		raw, err := r.lines.ReadString('\n')
		if err != nil {
			return zero, r.interrupted(err)
		}
		line := strings.TrimSuffix(strings.TrimSuffix(raw, "\n"), "\r")
		if line == "" {
			if len(data) == 0 {
				event = ""
				continue
			}
			payload := strings.Join(data, "\n")
			switch event {
			case "complete":
				r.finish(StreamCompleted)
				return zero, io.EOF
			case "error":
				return zero, r.failure([]byte(payload))
			case "", "next", "message":
				var value T
				if err := json.Unmarshal([]byte(payload), &value); err != nil {
					return zero, r.interrupted(err)
				}
				return value, nil
			}
			event = ""
			data = nil
			continue
		}
		if strings.HasPrefix(line, ":") {
			continue
		}
		if value, ok := strings.CutPrefix(line, "event:"); ok {
			event = strings.TrimSpace(value)
		}
		if value, ok := strings.CutPrefix(line, "data:"); ok {
			data = append(data, strings.TrimPrefix(value, " "))
		}
	}
}
