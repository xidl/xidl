package xidlgohttp

import (
	"errors"
	json "github.com/xidl/xidl/golang/xidl-go-codec"
	"net/http"
)

// HTTPFailure reuses an operation's HTTP exception status and JSON body.
type HTTPFailure struct {
	Status  int             `json:"status" xjson:"status"`
	Body    json.RawMessage `json:"body" xjson:"body"`
	Headers http.Header     `json:"-" xjson:"-"`
}

// EncodeHTTPError preserves application HTTP errors in an operation failure.
func EncodeHTTPError(err error) HTTPFailure {
	status, code := 500, 500
	var httpError *HttpError
	if errors.As(err, &httpError) {
		status, code = httpError.Status, httpError.Code
	}
	body, marshalErr := json.Marshal(map[string]any{"code": code, "msg": err.Error()})
	if marshalErr != nil {
		body = []byte(`{"code":500,"msg":"HTTP error encoding failed"}`)
		status = 500
	}
	return HTTPFailure{Status: status, Body: body}
}

// DecodeHTTPError retains an undeclared response as an HTTP error.
func DecodeHTTPError(failure HTTPFailure) error {
	var body struct {
		Code    int    `json:"code"`
		Message string `json:"msg"`
	}
	if json.Unmarshal(failure.Body, &body) == nil && body.Message != "" {
		return &HttpError{Status: failure.Status, Code: body.Code, Message: body.Message}
	}
	return NewHttpError(failure.Status, "undeclared HTTP exception: "+string(failure.Body))
}
