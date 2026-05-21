package rest_media_types

import (
	"context"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/gin-gonic/gin"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
)

type testRestMediaTypesService struct{}

func (testRestMediaTypesService) SubmitProfile(
	_ context.Context,
	req *RestMediaTypesApiSubmitProfileRequest,
) (*RestMediaTypesApiSubmitProfileResponse, error) {
	return &RestMediaTypesApiSubmitProfileResponse{
		Return:         req.Name + ":" + xidlgohttp.FormatUint32(req.Age),
		NormalizedName: strings.ToUpper(req.Name),
	}, nil
}

func (testRestMediaTypesService) GetMsgpackUser(
	_ context.Context,
	req *RestMediaTypesApiGetMsgpackUserRequest,
) (*RestMediaTypesApiGetMsgpackUserResponse, error) {
	return &RestMediaTypesApiGetMsgpackUserResponse{
		Return: "user:" + req.UserId,
		Score:  95,
	}, nil
}

func TestRestMediaTypes(t *testing.T) {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	RegisterRestMediaTypesApiHandler(r, testRestMediaTypesService{})
	server := httptest.NewServer(r)
	defer server.Close()

	client := NewRestMediaTypesApiClient(server.URL, server.Client(), xidlgohttp.ClientAuth{})

	submit, err := client.SubmitProfile(context.Background(), &RestMediaTypesApiSubmitProfileRequest{
		Name: "Taylor",
		Age:  42,
	})
	if err != nil {
		t.Fatalf("SubmitProfile failed: %v", err)
	}
	if submit.Return != "Taylor:42" {
		t.Fatalf("unexpected return: %q", submit.Return)
	}
	if submit.NormalizedName != "TAYLOR" {
		t.Fatalf("unexpected normalized name: %q", submit.NormalizedName)
	}

	msgpack, err := client.GetMsgpackUser(context.Background(), &RestMediaTypesApiGetMsgpackUserRequest{
		UserId: "u100",
	})
	if err != nil {
		t.Fatalf("GetMsgpackUser failed: %v", err)
	}
	if msgpack.Return != "user:u100" {
		t.Fatalf("unexpected msgpack return: %q", msgpack.Return)
	}
	if msgpack.Score != 95 {
		t.Fatalf("unexpected score: %d", msgpack.Score)
	}
}
