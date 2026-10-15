module test

go 1.25

replace github.com/xidl/xidl/golang/xidl-go-rest => {{GOLANG_XIDL_GO_REST_PATH}}
replace github.com/xidl/xidl/golang/xidl-go => {{GOLANG_XIDL_GO_PATH}}
replace github.com/xidl/xidl/golang/xidl-go-codec => {{GOLANG_XIDL_GO_CODEC_PATH}}

require (
	github.com/xidl/xidl/golang/xidl-go-rest v0.0.0
	github.com/gin-gonic/gin v1.12.0
)
