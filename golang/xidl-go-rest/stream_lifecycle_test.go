package xidlgohttp

import (
	"context"
	"errors"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

type countedBody struct {
	io.Reader
	closes int
}

func (b *countedBody) Close() error { b.closes++; return nil }

func TestReadersOwnTerminalOutcomeAndRelease(t *testing.T) {
	cases := []struct {
		name, wire string
		outcome    StreamOutcome
	}{
		{"completed", ": heartbeat\r\n\r\nevent:\r\ndata: 0\r\n\r\nevent: complete\r\ndata: {}\r\n\r\ndata: 99\n\n", StreamCompleted},
		{"failed", "data: 0\n\nevent: error\ndata: {\"status\":409,\"body\":{\"code\":409,\"msg\":\"locked\"}}\n\ndata: 99\n\n", StreamFailed},
		{"bodyless failure", "data: 0\n\nevent: error\ndata: {\"status\":304,\"body\":null}\n\n", StreamFailed},
		{"missing error body", "data: 0\n\nevent: error\ndata: {\"status\":409}\n\n", StreamInterrupted},
		{"truncated", "data: 0\n\nevent: complete\ndata: {}\n", StreamInterrupted},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			body := &countedBody{Reader: strings.NewReader(c.wire)}
			reader := NewSSEStreamReader[int](body)
			value, err := reader.Read()
			if err != nil || value != 0 {
				t.Fatalf("first: %v %v", value, err)
			}
			_, err = reader.Read()
			if c.outcome == StreamCompleted && !errors.Is(err, io.EOF) {
				t.Fatal(err)
			}
			if c.outcome == StreamInterrupted {
				var interrupted *StreamInterruptedError
				if !errors.As(err, &interrupted) {
					t.Fatalf("missing interruption: %v", err)
				}
				if errors.Is(err, io.EOF) {
					t.Fatal("truncation must not look like normal end of input")
				}
			}
			if reader.Outcome() != c.outcome {
				t.Fatal(reader.Outcome())
			}
			_ = reader.Close()
			_ = reader.Close()
			if reader.Outcome() != c.outcome || body.closes != 1 {
				t.Fatalf("cleanup: %v closes=%d", reader.Outcome(), body.closes)
			}
			if _, err = reader.Read(); !errors.Is(err, io.EOF) {
				t.Fatal(err)
			}
		})
	}
	body := &countedBody{Reader: strings.NewReader("data: 99\n\nevent: complete\ndata: {}\n\n")}
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	reader := NewSSEStreamReader[int](body).WithContext(ctx)
	if _, err := reader.Read(); !errors.Is(err, context.Canceled) {
		t.Fatal(err)
	}
	if reader.Outcome() != StreamCancelled || body.closes != 1 {
		t.Fatal(reader.Outcome(), body.closes)
	}
	large := strings.Repeat("x", 70_000)
	r := NewSSEStreamReader[string](io.NopCloser(strings.NewReader("data: \"" + large + "\"\n\nevent: complete\ndata: {}\n\n")))
	if value, err := r.Read(); err != nil || value != large {
		t.Fatalf("large item: %v", err)
	}
	_ = r.Close()
}

func TestWritersCommitOnlyPreparedFramesAndFlush(t *testing.T) {
	for _, started := range []bool{false, true} {
		recorder := httptest.NewRecorder()
		writer := NewSSEServerStreamWriter[int](recorder)
		if started {
			if err := writer.Write(0); err != nil {
				t.Fatal(err)
			}
			if !recorder.Flushed {
				t.Fatal("first item was buffered")
			}
		}
		if err := writer.Fail(EncodeHTTPError(NewHttpError(409, "locked"))); err != nil {
			t.Fatal(err)
		}
		_ = writer.Close()
		_ = writer.Close()
		wire := recorder.Body.String()
		if writer.Outcome() != StreamFailed {
			t.Fatal(writer.Outcome())
		}
		if started {
			if recorder.Code != 200 || strings.Count(wire, "event: error") != 1 || strings.Contains(wire, "event: complete") {
				t.Fatal(wire)
			}
		} else if recorder.Code != 409 || strings.Contains(wire, "event:") {
			t.Fatal(recorder.Code, wire)
		}
	}
	recorder := httptest.NewRecorder()
	writer := NewNDJSONServerStreamWriter[any](recorder)
	for _, value := range []any{0, false, "", nil} {
		if err := writer.Write(value); err != nil {
			t.Fatal(err)
		}
	}
	if err := writer.Close(); err != nil {
		t.Fatal(err)
	}
	reader := NewNDJSONStreamReader[any](io.NopCloser(strings.NewReader(recorder.Body.String())))
	for range 4 {
		if _, err := reader.Read(); err != nil {
			t.Fatal(err)
		}
	}
	if _, err := reader.Read(); !errors.Is(err, io.EOF) {
		t.Fatal(err)
	}
	if reader.Outcome() != StreamCompleted {
		t.Fatal(reader.Outcome())
	}
	missing := NewNDJSONStreamReader[int](io.NopCloser(strings.NewReader("{\"t\":\"next\"}\n")))
	if _, err := missing.Read(); err == nil || missing.Outcome() != StreamInterrupted {
		t.Fatal(err)
	}
	truncated := NewNDJSONStreamReader[int](io.NopCloser(strings.NewReader("{\"t\":\"next\",\"data\":0}\n")))
	_, _ = truncated.Read()
	if _, err := truncated.Read(); errors.Is(err, io.EOF) || truncated.Outcome() != StreamInterrupted {
		t.Fatalf("truncated input appeared complete: %v", err)
	}
}

type streamTransport func(*http.Request) (*http.Response, error)

func (f streamTransport) RoundTrip(r *http.Request) (*http.Response, error) { return f(r) }

func TestUploadRetainsEarlyHTTPFailureAndNeverCompletesAfterEncodingFailure(t *testing.T) {
	client := &http.Client{Transport: streamTransport(func(*http.Request) (*http.Response, error) {
		return &http.Response{StatusCode: 409, Body: io.NopCloser(strings.NewReader("locked"))}, nil
	})}
	request, _ := http.NewRequest("POST", "http://local/import", nil)
	declared := errors.New("declared Conflict")
	writer := NewClientStreamWriter[int, int](context.Background(), client, request, func(*http.Response) (int, error) { return 0, declared })
	for range 2 {
		if _, err := writer.Close(); err != declared {
			t.Fatalf("lost HTTP failure: %v", err)
		}
	}
	var wire string
	client.Transport = streamTransport(func(r *http.Request) (*http.Response, error) {
		data, err := io.ReadAll(r.Body)
		wire = string(data)
		if err != nil {
			return nil, err
		}
		return &http.Response{StatusCode: 200, Body: io.NopCloser(strings.NewReader("ok"))}, nil
	})
	writer2 := NewClientStreamWriter[any, int](context.Background(), client, request, func(*http.Response) (int, error) { return 1, nil })
	if err := writer2.Write(0); err != nil {
		t.Fatal(err)
	}
	if err := writer2.Write(make(chan int)); err == nil {
		t.Fatal("encoding must fail")
	}
	if _, err := writer2.Close(); err == nil {
		t.Fatal("partial upload must fail")
	}
	if !strings.Contains(wire, `"t":"error"`) || strings.Contains(wire, `"t":"complete"`) {
		t.Fatal(wire)
	}
}
