package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type MyForm struct{}

func (s *MyForm) Submit(ctx context.Context, req *FormServiceSubmitRequest) (*FormServiceSubmitResponse, error) {
	return &FormServiceSubmitResponse{Return: fmt.Sprintf("Received %s age %d", req.Name, req.Age)}, nil
}
func main() {
	r := gin.Default()
	svc := &MyForm{}
	RegisterFormServiceHandler(r, svc)
	http.ListenAndServe(fmt.Sprintf(":%s", os.Getenv("PORT")), r)
}
