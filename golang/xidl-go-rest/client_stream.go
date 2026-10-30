package xidlgohttp

import (
	"context"
	"io"
	"net/http"
	"sync"

	json "github.com/xidl/xidl/golang/xidl-go-codec"
)

type ClientStreamWriter[TReq any, TResp any] struct {
	encoder   *json.Encoder
	writer    *io.PipeWriter
	done      chan result[TResp]
	closeOnce sync.Once
	final     result[TResp]
	outcome   StreamOutcome
	cause     error
}

type result[T any] struct {
	value T
	err   error
}

func NewClientStreamWriter[TReq any, TResp any](
	ctx context.Context,
	client *http.Client,
	req *http.Request,
	decode func(*http.Response) (TResp, error),
) *ClientStreamWriter[TReq, TResp] {
	pr, pw := io.Pipe()
	req = req.WithContext(ctx)
	req.Body = io.NopCloser(pr)
	req.Header.Set("Content-Type", "application/x-ndjson")
	req.Header.Set("X-Xidl-Stream-Mode", "client")
	req.Header.Set("X-Xidl-Stream-Version", "1")
	done := make(chan result[TResp], 1)
	go func() {
		defer pr.Close()
		resp, err := client.Do(req)
		if err != nil {
			var zero TResp
			done <- result[TResp]{value: zero, err: err}
			return
		}
		defer resp.Body.Close()
		value, err := decode(resp)
		done <- result[TResp]{value: value, err: err}
	}()
	return &ClientStreamWriter[TReq, TResp]{
		encoder: json.NewEncoder(pw),
		writer:  pw,
		done:    done,
	}
}

// Write prepares a complete item before touching the transport.
func (w *ClientStreamWriter[TReq, TResp]) Write(value TReq) error {
	if w.outcome != "" {
		return io.ErrClosedPipe
	}
	data, err := json.Marshal(value)
	if err != nil {
		w.outcome = StreamFailed
		w.cause = err
		_ = w.encoder.Encode(map[string]any{"t": "error", "error": EncodeHTTPError(err)})
		_ = w.writer.Close()
		return err
	}
	err = w.encoder.Encode(map[string]any{"t": "next", "data": json.RawMessage(data)})
	if err != nil {
		w.cause = err
		w.outcome = StreamInterrupted
		_ = w.writer.CloseWithError(err)
	}
	return err
}

// Close sends at most one terminal frame and retains the final HTTP result.
func (w *ClientStreamWriter[TReq, TResp]) Close() (TResp, error) {
	w.closeOnce.Do(func() {
		if w.outcome == "" {
			w.cause = w.encoder.Encode(map[string]any{"t": "complete"})
			if w.cause != nil {
				w.outcome = StreamInterrupted
			} else {
				w.outcome = StreamCompleted
			}
		}
		_ = w.writer.CloseWithError(w.cause)
		w.final = <-w.done
		if w.final.err == nil && w.cause != nil {
			w.final.err = w.cause
		}
		if w.final.err != nil && w.outcome == StreamCompleted {
			w.outcome = StreamFailed
		}
	})
	return w.final.value, w.final.err
}

func (w *ClientStreamWriter[TReq, TResp]) Outcome() StreamOutcome { return w.outcome }
