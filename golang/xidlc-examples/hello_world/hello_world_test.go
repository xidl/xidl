package hello_world

import (
	"context"
	"net/http/httptest"
	"testing"

	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-http"
)

type testHelloWorldService struct{}

func (testHelloWorldService) SayHello(_ context.Context, req *HelloWorldSayHelloRequest) (*HelloWorldSayHelloResponse, error) {
	if req.Name == "" {
		panic("name should not be empty")
	}
	return &HelloWorldSayHelloResponse{}, nil
}

func TestHelloWorld(t *testing.T) {
	server := httptest.NewServer(NewHelloWorldHandler(testHelloWorldService{}))
	defer server.Close()

	client := NewHelloWorldClient(server.URL, server.Client(), xidlgohttp.ClientAuth{})
	if _, err := client.SayHello(context.Background(), &HelloWorldSayHelloRequest{
		Name: "Taylor",
	}); err != nil {
		t.Fatalf("SayHello failed: %v", err)
	}
}
