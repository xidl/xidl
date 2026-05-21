package hello_world

import (
	"context"
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

func TestHelloWorld(t *testing.T) {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	RegisterHelloWorldHandler(r, testHelloWorldService{})
	server := httptest.NewServer(r)
	defer server.Close()

	client := NewHelloWorldClient(server.URL, server.Client(), xidlgohttp.ClientAuth{})
	if _, err := client.SayHello(context.Background(), &HelloWorldSayHelloRequest{
		Name: "Taylor",
	}); err != nil {
		t.Fatalf("SayHello failed: %v", err)
	}
}
