package xidlgohttp

import (
	"errors"
	"fmt"
	json "github.com/xidl/xidl/golang/xidl-go-codec"
	"net/http"
)

type streamWriter struct {
	response  http.ResponseWriter
	sse       bool
	committed bool
	outcome   StreamOutcome
	seq       uint64
}

func (w *streamWriter) Committed() bool        { return w.committed }
func (w *streamWriter) Outcome() StreamOutcome { return w.outcome }
func (w *streamWriter) send(kind string, data []byte) error {
	if w.outcome != "" {
		return errors.New("stream already terminated")
	}
	var frame []byte
	var err error
	if w.sse {
		frame = []byte(fmt.Sprintf("event: %s\ndata: %s\n\n", kind, data))
	} else {
		value := map[string]any{"t": kind}
		if kind == "next" {
			value["data"] = json.RawMessage(data)
			value["seq"] = w.seq + 1
		}
		if kind == "error" {
			value["error"] = json.RawMessage(data)
		}
		frame, err = json.Marshal(value)
		if err != nil {
			return err
		}
		frame = append(frame, '\n')
	}
	if !w.committed {
		mime := "application/x-ndjson"
		if w.sse {
			mime = "text/event-stream"
			w.response.Header().Set("Cache-Control", "no-cache")
		}
		w.response.Header().Set("Content-Type", mime)
		w.response.Header().Set("X-Xidl-Stream-Mode", "server")
		w.response.Header().Set("X-Xidl-Stream-Version", "1")
		w.committed = true
	}
	if _, err = w.response.Write(frame); err != nil {
		w.outcome = StreamInterrupted
		return err
	}
	if err = http.NewResponseController(w.response).Flush(); err != nil {
		w.outcome = StreamInterrupted
		return err
	}
	if kind == "next" {
		w.seq++
	}
	if kind == "complete" {
		w.outcome = StreamCompleted
	}
	if kind == "error" {
		w.outcome = StreamFailed
	}
	return nil
}

// Fail uses the normal HTTP exception response before the first output.
func (w *streamWriter) Fail(failure StreamFailure) error {
	if w.outcome != "" {
		return nil
	}
	if !w.committed {
		w.outcome = StreamFailed
		w.response.Header().Set("Content-Type", "application/json")
		w.response.WriteHeader(failure.Status)
		_, err := w.response.Write(failure.Body)
		return err
	}
	data, err := json.Marshal(failure)
	if err != nil {
		w.outcome = StreamInterrupted
		return err
	}
	return w.send("error", data)
}

func (w *streamWriter) Close() error {
	if w.outcome != "" {
		return nil
	}
	return w.send("complete", []byte(`{}`))
}

type NDJSONServerStreamWriter[T any] struct{ *streamWriter }

func NewNDJSONServerStreamWriter[T any](w http.ResponseWriter) *NDJSONServerStreamWriter[T] {
	return &NDJSONServerStreamWriter[T]{&streamWriter{response: w}}
}
func (s *NDJSONServerStreamWriter[T]) Write(value T) error {
	data, err := json.Marshal(value)
	if err != nil {
		if failure := s.Fail(EncodeStreamError(err)); failure != nil {
			return failure
		}
		return err
	}
	return s.send("next", data)
}

type SSEServerStreamWriter[T any] struct{ *streamWriter }

func NewSSEServerStreamWriter[T any](w http.ResponseWriter) *SSEServerStreamWriter[T] {
	return &SSEServerStreamWriter[T]{&streamWriter{response: w, sse: true}}
}
func (s *SSEServerStreamWriter[T]) Write(value T) error {
	data, err := json.Marshal(value)
	if err != nil {
		if failure := s.Fail(EncodeStreamError(err)); failure != nil {
			return failure
		}
		return err
	}
	return s.send("next", data)
}
