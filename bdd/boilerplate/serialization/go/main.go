package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type MySerializationTest struct{}

func (s *MySerializationTest) GetString(ctx context.Context, req *SerializationTestGetStringRequest) (*SerializationTestGetStringResponse, error) {
	return &SerializationTestGetStringResponse{Return: "hello"}, nil
}
func (s *MySerializationTest) GetInt(ctx context.Context, req *SerializationTestGetIntRequest) (*SerializationTestGetIntResponse, error) {
	return &SerializationTestGetIntResponse{Return: 42}, nil
}
func (s *MySerializationTest) GetBool(ctx context.Context, req *SerializationTestGetBoolRequest) (*SerializationTestGetBoolResponse, error) {
	return &SerializationTestGetBoolResponse{Return: true}, nil
}
func (s *MySerializationTest) GetStruct(ctx context.Context, req *SerializationTestGetStructRequest) (*SerializationTestGetStructResponse, error) {
	return &SerializationTestGetStructResponse{Return: Item{Name: "world"}}, nil
}
func (s *MySerializationTest) EchoString(ctx context.Context, req *SerializationTestEchoStringRequest) (*SerializationTestEchoStringResponse, error) {
	return &SerializationTestEchoStringResponse{Return: req.Value}, nil
}
func (s *MySerializationTest) EchoStruct(ctx context.Context, req *SerializationTestEchoStructRequest) (*SerializationTestEchoStructResponse, error) {
	return &SerializationTestEchoStructResponse{Return: req.Value}, nil
}
func main() {
	r := gin.Default()
	svc := &MySerializationTest{}
	RegisterSerializationTestHandler(r, svc)
	http.ListenAndServe(fmt.Sprintf(":%s", os.Getenv("PORT")), r)
}
