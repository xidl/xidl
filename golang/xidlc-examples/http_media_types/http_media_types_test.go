package http_media_types

import (
	"context"
	"net/http/httptest"
	"strings"
	"testing"

	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-http"
)

type testHttpMediaTypesService struct{}

func (testHttpMediaTypesService) SubmitProfile(
	_ context.Context,
	req *HttpMediaTypesApiSubmitProfileRequest,
) (*HttpMediaTypesApiSubmitProfileResponse, error) {
	return &HttpMediaTypesApiSubmitProfileResponse{
		Return:         req.Name + ":" + xidlgohttp.FormatUint32(req.Age),
		NormalizedName: strings.ToUpper(req.Name),
	}, nil
}

func (testHttpMediaTypesService) GetMsgpackUser(
	_ context.Context,
	req *HttpMediaTypesApiGetMsgpackUserRequest,
) (*HttpMediaTypesApiGetMsgpackUserResponse, error) {
	return &HttpMediaTypesApiGetMsgpackUserResponse{
		Return: "user:" + req.UserId,
		Score:  95,
	}, nil
}

func TestHttpMediaTypes(t *testing.T) {
	server := httptest.NewServer(NewHttpMediaTypesApiHandler(testHttpMediaTypesService{}))
	defer server.Close()

	client := NewHttpMediaTypesApiClient(server.URL, server.Client(), xidlgohttp.ClientAuth{})

	submit, err := client.SubmitProfile(context.Background(), &HttpMediaTypesApiSubmitProfileRequest{
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

	msgpack, err := client.GetMsgpackUser(context.Background(), &HttpMediaTypesApiGetMsgpackUserRequest{
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
