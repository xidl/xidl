package main

import (
	"context"
	"log"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
)

type svc struct{}

func (svc) Control(
	ctx context.Context,
	stream *xidlgohttp.WSBidiStream[WsControlControlStreamIn, WsControlControlStreamOut],
) error {
	for {
		in, err := stream.Read()
		if err != nil {
			return nil
		}
		if err := stream.Write(WsControlControlStreamOut{
			Event: in.Opcode + ":ack",
			Data:  in.Payload,
		}); err != nil {
			return err
		}
	}
}

func main() {
	r := gin.Default()
	RegisterWsControlHandler(r, svc{})
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	log.Println("listening", port)
	_ = http.ListenAndServe("127.0.0.1:"+port, r)
}
