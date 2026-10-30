module xidl.test/integer-masks

go 1.25.0

require masks v0.0.0

replace masks => ./masks

replace github.com/xidl/xidl/golang/xidl-go => {{GOLANG_XIDL_GO_PATH}}

replace github.com/xidl/xidl/golang/xidl-go-codec => {{GOLANG_XIDL_GO_CODEC_PATH}}
