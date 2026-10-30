package xidlgohttp

import (
	"fmt"
	"io"
	"net/http"
)

// ReadHTTPError decodes a non-success response through the operation exception codec.
func ReadHTTPError(response *http.Response, decode func(HTTPFailure) error) error {
	body, err := io.ReadAll(response.Body)
	if err != nil {
		return fmt.Errorf("HTTP error body: %w", err)
	}
	return decode(HTTPFailure{Status: response.StatusCode, Body: body, Headers: response.Header})
}
