# Changelog

## [0.79.1](https://github.com/xidl/xidl/compare/golang/xidl-go-rest/v0.79.0...golang/xidl-go-rest/v0.79.1) (2026-06-12)


### Bug Fixes

* enable subpath tags for Go modules and update internal dependencies using release-please ([4a5591b](https://github.com/xidl/xidl/commit/4a5591b1f969c5ae73bf48821d3cb2e06193c073))

## [0.79.0](https://github.com/xidl/xidl/compare/golang/xidl-go-rest/v0.78.1...golang/xidl-go-rest/v0.79.0) (2026-06-12)


### Features

* **go:** add support for [@cors](https://github.com/cors) annotation ([253ce87](https://github.com/xidl/xidl/commit/253ce874cd2aed65f2362e4d1c4fb6d0c0cd8d0e))
* **golang:** use xidl-go-json codec in go-rest and emit xjson tags in xidlc ([772f7ff](https://github.com/xidl/xidl/commit/772f7ffb1916975d5d896fd36ca0c033cb51a2d4))
* **go:** migrate go-rest generator from net/http to gin ([d682fc9](https://github.com/xidl/xidl/commit/d682fc9615d43f42a56e3abb8e5b9368bf487b5b))
* **http:** implement raw text serialization for primitive types and update BDD tests ([a72654f](https://github.com/xidl/xidl/commit/a72654f59651c7866e78ee12e7b09fb52497cd5b))
* **http:** standardize error response format to {code, msg} and add BDD bad path tests ([a7cc8d3](https://github.com/xidl/xidl/commit/a7cc8d320968e77a05cdaf8de304264ac423b1ca))
* rename *http to *rest ([a9fe5dd](https://github.com/xidl/xidl/commit/a9fe5dd13e426183ee5f8f061d2bb958b18fe91f))
* **rename:** rename typescript-json to typescript-codec and go-json to go-codec ([adb9880](https://github.com/xidl/xidl/commit/adb988056ddfd81472d2272c0c9f4893538ce719))


### Bug Fixes

* **go:** update GinWriteJSONError calls in examples to match new signature ([dca7e75](https://github.com/xidl/xidl/commit/dca7e758a1502969dafdbe50d94e27fb47342d62))
