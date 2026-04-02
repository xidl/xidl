package http_security

import (
	"context"
	"testing"

	"net/http/httptest"

	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-http"
)

type testHttpSecurityService struct{}

func (testHttpSecurityService) GetReport(ctx context.Context, req *HttpSecurityApiGetReportRequest) (*HttpSecurityApiGetReportResponse, error) {
	basic, ok := xidlgohttp.BasicAuthFromContext(ctx)
	if !ok || basic.Username != "alice" || basic.Password != "secret" {
		tpanic("basic auth missing from context")
	}
	return &HttpSecurityApiGetReportResponse{
		Return: req.Id + ":" + req.TraceId,
	}, nil
}

func (testHttpSecurityService) UpdateReport(ctx context.Context, req *HttpSecurityApiUpdateReportRequest) (*HttpSecurityApiUpdateReportResponse, error) {
	bearer, ok := xidlgohttp.BearerFromContext(ctx)
	if !ok || bearer != "oauth-token" {
		tpanic("oauth2 bearer missing from context")
	}
	return &HttpSecurityApiUpdateReportResponse{
		Return: req.Id + ":" + req.Body,
	}, nil
}

func (testHttpSecurityService) Health(_ context.Context, _ *HttpSecurityApiHealthRequest) (*HttpSecurityApiHealthResponse, error) {
	return &HttpSecurityApiHealthResponse{Return: "ok"}, nil
}

func TestHttpSecurity(t *testing.T) {
	server := httptest.NewServer(NewHttpSecurityApiHandler(testHttpSecurityService{}))
	defer server.Close()

	client := NewHttpSecurityApiClient(server.URL, server.Client(), xidlgohttp.ClientAuth{
		Basic: &xidlgohttp.BasicAuth{
			Username: "alice",
			Password: "secret",
		},
		Bearer: "oauth-token",
		APIKeys: []xidlgohttp.ApiKeyAuth{
			{Location: xidlgohttp.ApiKeyHeader, Name: "X-Org-Key", Value: "org-secret"},
		},
	})

	report, err := client.GetReport(context.Background(), &HttpSecurityApiGetReportRequest{
		Id:      "r-1",
		TraceId: "trace-9",
	})
	if err != nil {
		t.Fatalf("GetReport failed: %v", err)
	}
	if report.Return != "r-1:trace-9" {
		t.Fatalf("unexpected report response: %q", report.Return)
	}

	updated, err := client.UpdateReport(context.Background(), &HttpSecurityApiUpdateReportRequest{
		Id:   "r-1",
		Body: "patched",
	})
	if err != nil {
		t.Fatalf("UpdateReport failed: %v", err)
	}
	if updated.Return != "r-1:patched" {
		t.Fatalf("unexpected update response: %q", updated.Return)
	}

	health, err := client.Health(context.Background(), &HttpSecurityApiHealthRequest{})
	if err != nil {
		t.Fatalf("Health failed: %v", err)
	}
	if health.Return != "ok" {
		t.Fatalf("unexpected health response: %q", health.Return)
	}
}

func tpanic(message string) {
	panic(message)
}
