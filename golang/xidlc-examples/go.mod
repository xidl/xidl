module github.com/xidl/xidl/golang/xidlc-examples

go 1.25.0

require github.com/xidl/xidl/golang/xidl-go-rest v0.0.0

require (
	github.com/vmihailenco/msgpack/v5 v5.4.1 // indirect
	github.com/vmihailenco/tagparser/v2 v2.0.0 // indirect
)

replace github.com/xidl/xidl/golang/xidl-go => ../xidl-go

replace github.com/xidl/xidl/golang/xidl-go-rest => ../xidl-go-rest
