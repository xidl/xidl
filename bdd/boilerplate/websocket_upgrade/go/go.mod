module test-go-ws

go 1.25.0

require (
	github.com/gin-gonic/gin v1.12.0
	github.com/xidl/xidl/golang/xidl-go-rest v0.0.0
)

replace github.com/xidl/xidl/golang/xidl-go-rest => {{GOLANG_XIDL_GO_REST_PATH}}

replace github.com/xidl/xidl/golang/xidl-go-codec => {{GOLANG_XIDL_GO_CODEC_PATH}}
