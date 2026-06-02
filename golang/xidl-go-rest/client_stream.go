package xidlgohttp

import (
	"context"
	"io"
	"net/http"

	json "github.com/xidl/xidl/golang/xidl-go-codec"
)

type ClientStreamWriter[TReq any, TResp any] struct {
	encoder *json.Encoder
	writer  *io.PipeWriter
	done    chan result[TResp]
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

func (w *ClientStreamWriter[TReq, TResp]) Write(value TReq) error {
	return w.encoder.Encode(StreamFrame[TReq]{
		Type: "next",
		Data: value,
	})
}

func (w *ClientStreamWriter[TReq, TResp]) Close() (TResp, error) {
	var zero TResp
	_ = w.encoder.Encode(map[string]any{"t": "complete"})
	if err := w.writer.Close(); err != nil {
		return zero, err
	}
	out := <-w.done
	return out.value, out.err
}
