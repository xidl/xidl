package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
)

type MyStream struct{}

func (s *MyStream) Ticks(ctx context.Context, req *StreamingServiceTicksRequest, stream xidlgohttp.ServerStreamWriter[int32]) error {
	for i := int32(0); i < req.Count; i++ {
		if err := stream.Write(i); err != nil {
			return err
		}
	}
	return nil
}
func main() {
	r := gin.Default()
	svc := &MyStream{}
	RegisterStreamingServiceHandler(r, svc)
	http.ListenAndServe(fmt.Sprintf(":%s", os.Getenv("PORT")), r)
}
