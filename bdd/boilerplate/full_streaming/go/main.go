package main

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
)

type MyStreamService struct{}

func (s *MyStreamService) Alerts(ctx context.Context, req *HttpStreamApiAlertsRequest, stream xidlgohttp.ServerStreamWriter[string]) error {
	if _, ok := xidlgohttp.BasicAuthFromContext(ctx); !ok {
		return fmt.Errorf("basic auth missing")
	}
	stream.Write(req.District + ":1")
	stream.Write(req.District + ":2")
	return nil
}

func (s *MyStreamService) UploadAsset(ctx context.Context, stream *xidlgohttp.ClientStreamReader[HttpStreamApiUploadAssetRequest]) (*HttpStreamApiUploadAssetResponse, error) {
	if _, ok := xidlgohttp.BearerFromContext(ctx); !ok {
		return nil, fmt.Errorf("bearer auth missing")
	}
	assetID := ""
	for {
		item, err := stream.Read()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, err
		}
		assetID = item.AssetId
	}
	return &HttpStreamApiUploadAssetResponse{Return: "uploaded:" + assetID}, nil
}

func main() {
	r := gin.Default()
	RegisterHttpStreamApiHandler(r, &MyStreamService{})
	http.ListenAndServe(fmt.Sprintf(":%s", os.Getenv("PORT")), r)
}
