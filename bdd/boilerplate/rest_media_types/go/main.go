package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"strings"

	"github.com/gin-gonic/gin"
)

type MyRestMediaTypesService struct{}

func (s *MyRestMediaTypesService) SubmitProfile(ctx context.Context, req *RestMediaTypesApiSubmitProfileRequest) (*RestMediaTypesApiSubmitProfileResponse, error) {
	return &RestMediaTypesApiSubmitProfileResponse{
		Return:         fmt.Sprintf("%s:%d", req.Name, req.Age),
		NormalizedName: strings.ToUpper(req.Name),
	}, nil
}

func (s *MyRestMediaTypesService) GetMsgpackUser(ctx context.Context, req *RestMediaTypesApiGetMsgpackUserRequest) (*RestMediaTypesApiGetMsgpackUserResponse, error) {
	return &RestMediaTypesApiGetMsgpackUserResponse{
		Return: "user:" + req.UserId,
		Score:  95,
	}, nil
}

func main() {
	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()
	svc := &MyRestMediaTypesService{}
	RegisterRestMediaTypesApiHandler(r, svc)
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	fmt.Printf("Go server starting on port %s\n", port)
	http.ListenAndServe(fmt.Sprintf(":%s", port), r)
}
