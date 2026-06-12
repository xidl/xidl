package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type MyHelloWorld struct{}

func (s *MyHelloWorld) Hello(ctx context.Context, req *HelloWorldHelloRequest) (*HelloWorldHelloResponse, error) {
	return &HelloWorldHelloResponse{Return: "Hello BDD"}, nil
}
func (s *MyHelloWorld) Echo(ctx context.Context, req *HelloWorldEchoRequest) (*HelloWorldEchoResponse, error) {
	return &HelloWorldEchoResponse{Return: req.Msg}, nil
}
func main() {
	r := gin.Default()
	svc := &MyHelloWorld{}
	RegisterHelloWorldHandler(r, svc)
	http.ListenAndServe(fmt.Sprintf(":%s", os.Getenv("PORT")), r)
}
