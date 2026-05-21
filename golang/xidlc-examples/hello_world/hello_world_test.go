package hello_world

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
)

type testHelloWorldService struct{}

func (testHelloWorldService) SayHello(_ context.Context, req *HelloWorldSayHelloRequest) (*HelloWorldSayHelloResponse, error) {
	if req.Name == "" {
		panic("name should not be empty")
	}
	return &HelloWorldSayHelloResponse{}, nil
}

func (testHelloWorldService) Trusted(_ context.Context, _ *HelloWorldTrustedRequest) (*HelloWorldTrustedResponse, error) {
	return &HelloWorldTrustedResponse{}, nil
}

func TestHelloWorld(t *testing.T) {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	RegisterHelloWorldHandler(r, testHelloWorldService{})
	server := httptest.NewServer(r)
	defer server.Close()

	// Test CORS Wildcard (with credentials)
	{
		req, _ := http.NewRequest("OPTIONS", server.URL+"/hello", nil)
		req.Header.Set("Origin", "http://example.com")
		req.Header.Set("Access-Control-Request-Method", "POST")
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			t.Fatalf("OPTIONS request failed: %v", err)
		}
		if resp.Header.Get("Access-Control-Allow-Origin") != "http://example.com" {
			t.Fatalf("unexpected Access-Control-Allow-Origin: %q", resp.Header.Get("Access-Control-Allow-Origin"))
		}
	}

	// Test CORS Specific Origin
	{
		// Should allow trusted origin
		req, _ := http.NewRequest("OPTIONS", server.URL+"/trusted", nil)
		req.Header.Set("Origin", "http://trust.me")
		req.Header.Set("Access-Control-Request-Method", "GET")
		resp, _ := http.DefaultClient.Do(req)
		if resp.Header.Get("Access-Control-Allow-Origin") != "http://trust.me" {
			t.Fatalf("expected Access-Control-Allow-Origin: http://trust.me, got %q", resp.Header.Get("Access-Control-Allow-Origin"))
		}

		// Should NOT allow untrusted origin
		req, _ = http.NewRequest("OPTIONS", server.URL+"/trusted", nil)
		req.Header.Set("Origin", "http://evil.com")
		req.Header.Set("Access-Control-Request-Method", "GET")
		resp, _ = http.DefaultClient.Do(req)
		if resp.Header.Get("Access-Control-Allow-Origin") != "" {
			t.Fatalf("expected NO Access-Control-Allow-Origin for evil.com, got %q", resp.Header.Get("Access-Control-Allow-Origin"))
		}
	}

	client := NewHelloWorldClient(server.URL, server.Client(), xidlgohttp.ClientAuth{})
	if _, err := client.SayHello(context.Background(), &HelloWorldSayHelloRequest{
		Name: "Taylor",
	}); err != nil {
		t.Fatalf("SayHello failed: %v", err)
	}
}
