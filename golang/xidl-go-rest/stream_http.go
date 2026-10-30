package xidlgohttp

import (
	"fmt"
	"io"
	"net/http"
)

// DecodeStreamHTTPError uses the same operation decoder before any stream output.
func DecodeStreamHTTPError(response *http.Response, decode func(StreamFailure) error) error {
	body, err := io.ReadAll(response.Body)
	if err != nil {
		return fmt.Errorf("stream HTTP error body: %w", err)
	}
	return decode(StreamFailure{Status: response.StatusCode, Body: body})
}
