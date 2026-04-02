package xidlgohttp

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
)

type StreamFrame[T any] struct {
	Type  string `json:"t"`
	Seq   uint64 `json:"seq,omitempty"`
	Data  T      `json:"data,omitempty"`
	Error any    `json:"error,omitempty"`
}

type ServerStreamWriter[T any] interface {
	Write(T) error
	Close() error
}

type NDJSONServerStreamWriter[T any] struct {
	w   io.Writer
	seq uint64
}

func NewNDJSONServerStreamWriter[T any](w http.ResponseWriter) *NDJSONServerStreamWriter[T] {
	w.Header().Set("Content-Type", "application/x-ndjson")
	w.Header().Set("X-Xidl-Stream-Mode", "server")
	w.Header().Set("X-Xidl-Stream-Version", "1")
	return &NDJSONServerStreamWriter[T]{w: w}
}

func (s *NDJSONServerStreamWriter[T]) Write(value T) error {
	s.seq++
	return json.NewEncoder(s.w).Encode(StreamFrame[T]{
		Type: "next",
		Seq:  s.seq,
		Data: value,
	})
}

func (s *NDJSONServerStreamWriter[T]) Close() error {
	return json.NewEncoder(s.w).Encode(map[string]any{
		"t": "complete",
	})
}

type SSEServerStreamWriter[T any] struct {
	w io.Writer
}

func NewSSEServerStreamWriter[T any](w http.ResponseWriter) *SSEServerStreamWriter[T] {
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.Header().Set("X-Xidl-Stream-Mode", "server")
	w.Header().Set("X-Xidl-Stream-Version", "1")
	return &SSEServerStreamWriter[T]{w: w}
}

func (s *SSEServerStreamWriter[T]) Write(value T) error {
	payload, err := json.Marshal(value)
	if err != nil {
		return err
	}
	_, err = fmt.Fprintf(s.w, "event: next\ndata: %s\n\n", payload)
	return err
}

func (s *SSEServerStreamWriter[T]) Close() error {
	_, err := io.WriteString(s.w, "event: complete\n\n")
	return err
}

type ServerStreamReader[T any] struct {
	body io.ReadCloser
	dec  *json.Decoder
}

func NewNDJSONStreamReader[T any](body io.ReadCloser) *ServerStreamReader[T] {
	return &ServerStreamReader[T]{
		body: body,
		dec:  json.NewDecoder(body),
	}
}

func (r *ServerStreamReader[T]) Read() (T, error) {
	var zero T
	var frame StreamFrame[T]
	if err := r.dec.Decode(&frame); err != nil {
		return zero, err
	}
	if frame.Type == "complete" {
		return zero, io.EOF
	}
	if frame.Type != "next" {
		return zero, errors.New("unexpected frame type: " + frame.Type)
	}
	return frame.Data, nil
}

func (r *ServerStreamReader[T]) Close() error {
	return r.body.Close()
}

type SSEStreamReader[T any] struct {
	body io.ReadCloser
	scan *bufio.Scanner
}

func NewSSEStreamReader[T any](body io.ReadCloser) *SSEStreamReader[T] {
	return &SSEStreamReader[T]{
		body: body,
		scan: bufio.NewScanner(body),
	}
}

func (r *SSEStreamReader[T]) Read() (T, error) {
	var zero T
	var event string
	var payload string
	for r.scan.Scan() {
		line := r.scan.Text()
		if line == "" {
			if event == "complete" {
				return zero, io.EOF
			}
			if event == "next" {
				var value T
				if err := json.Unmarshal([]byte(payload), &value); err != nil {
					return zero, err
				}
				return value, nil
			}
			event = ""
			payload = ""
			continue
		}
		if len(line) > 7 && line[:7] == "event: " {
			event = line[7:]
		}
		if len(line) > 6 && line[:6] == "data: " {
			payload = line[6:]
		}
	}
	if err := r.scan.Err(); err != nil {
		return zero, err
	}
	return zero, io.EOF
}

func (r *SSEStreamReader[T]) Close() error {
	return r.body.Close()
}

type ClientStreamReader[T any] struct {
	ctx context.Context
	dec *json.Decoder
}

func NewClientStreamReader[T any](ctx context.Context, body io.Reader) *ClientStreamReader[T] {
	return &ClientStreamReader[T]{
		ctx: ctx,
		dec: json.NewDecoder(body),
	}
}

func (r *ClientStreamReader[T]) Read() (T, error) {
	var zero T
	select {
	case <-r.ctx.Done():
		return zero, r.ctx.Err()
	default:
	}
	var frame StreamFrame[T]
	if err := r.dec.Decode(&frame); err != nil {
		return zero, err
	}
	if frame.Type == "complete" {
		return zero, io.EOF
	}
	if frame.Type != "next" {
		return zero, errors.New("unexpected frame type: " + frame.Type)
	}
	return frame.Data, nil
}
