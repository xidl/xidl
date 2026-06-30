# Changelog

## [0.83.1](https://github.com/xidl/xidl/compare/v0.83.0...v0.83.1) (2026-06-30)


### Bug Fixes

* **go-rest:** skip content-type checks without @Consumes ([a980e6c](https://github.com/xidl/xidl/commit/a980e6cf66670af764b7654e9798ad41675b8c88))

## [0.83.0](https://github.com/xidl/xidl/compare/v0.82.0...v0.83.0) (2026-06-26)


### Features

* **codec:** update typescript-xidl-codec ([0a756ae](https://github.com/xidl/xidl/commit/0a756ae05ae0602821d39c4d0cd811f63d2307b4))
* remove useless code ([0ba29ac](https://github.com/xidl/xidl/commit/0ba29acf98e11d64153fc4682ecf4050e7bfd921))
* **xidlc:** remove python support ([3ff1644](https://github.com/xidl/xidl/commit/3ff1644eed3b93dcb58f8de491488d382c393c0f))
* **xidlc:** remove typescript server support ([6d3c3f8](https://github.com/xidl/xidl/commit/6d3c3f8d78b25a43e5975f8c08bec2b03d7a9537))


### Bug Fixes

* **xidlc:** remove println before format ts ([c62621a](https://github.com/xidl/xidl/commit/c62621a19f5ae37adca90c4b73901656ae59b274))

## [0.82.0](https://github.com/xidl/xidl/compare/v0.81.0...v0.82.0) (2026-06-22)


### Features

* add [@cookie](https://github.com/cookie) support for http generators ([50c005b](https://github.com/xidl/xidl/commit/50c005b073f425a0fb24e5ec6d767445cac0443b))
* add [@http](https://github.com/http)(rename) for field serialization ([d7cb3c8](https://github.com/xidl/xidl/commit/d7cb3c88783f0b115d585a4fd7acbb8120b21697))
* add [@name](https://github.com/name) field rename ([bef6497](https://github.com/xidl/xidl/commit/bef64973f20eae1f42eab971757a2049ba0945ec))
* add [@skip](https://github.com/skip) annotation to support skipping fields during serialization ([7cbf6c8](https://github.com/xidl/xidl/commit/7cbf6c80365acf4f9f50c89e6547d973f8c2a6c9))
* add CD workflow and improve release automation ([755d4be](https://github.com/xidl/xidl/commit/755d4be6068188b2763e6976643a61282d1a7d90))
* add default Rust derives for struct and enum ([b25434b](https://github.com/xidl/xidl/commit/b25434bb9006bad056eabe917801506980521b3f))
* add doc comment to annotation ([bb1e191](https://github.com/xidl/xidl/commit/bb1e1918d14cc56657777bdf598a031cce209af6))
* add Homebrew formula and improve Windows package support ([4ad1161](https://github.com/xidl/xidl/commit/4ad116103c09680e680b1ecaca848cec4e88f477))
* add http hir layer ([0f3a307](https://github.com/xidl/xidl/commit/0f3a307537f96b9d0b9e56739098805c7c45f7fe))
* add hy2 ([b1c0100](https://github.com/xidl/xidl/commit/b1c0100b80d7155804a3001a4e4d53dcd7d6b04b))
* add keycloak ([47ca2e3](https://github.com/xidl/xidl/commit/47ca2e3f0752c834f021a113f92c6172b4e55555))
* add msgpack support ([bc5876c](https://github.com/xidl/xidl/commit/bc5876c5508844b06b5c6980161fa0e0c9a43051))
* Add OpenAPI 3.2 HTTP stream support ([7d285d6](https://github.com/xidl/xidl/commit/7d285d69b853102c66021cdf96a2a8346fe3f592))
* add ugprade ([ea48027](https://github.com/xidl/xidl/commit/ea48027f36a34d1befe8f82c8666b07bfbfa35f4))
* add upgrade annotation support ([b534408](https://github.com/xidl/xidl/commit/b5344084ccc9e4680c97a240b914b31330ee04cf))
* add xidl-api-discord ([c4cb195](https://github.com/xidl/xidl/commit/c4cb195486619caf54d21f73277934394ff93916))
* add xidl-api-github and openapi importer ([d0475ba](https://github.com/xidl/xidl/commit/d0475bae7ef5a5bfa1eaee26fd749647ef6a18b4))
* add xidl-apis-reddit ([5274a9c](https://github.com/xidl/xidl/commit/5274a9cf1c506c8f249e0d759f4fa7423424750f))
* align unary http and http security mapping ([c0dad07](https://github.com/xidl/xidl/commit/c0dad0728aa735267ecc3715d4ef04871588c015))
* apply [@name](https://github.com/name) to rust struct fields ([ebe42f0](https://github.com/xidl/xidl/commit/ebe42f0efb5ec2a5a69a8576738a0ef66218ba43))
* **axu,:** impl basic-auth ([43ecad3](https://github.com/xidl/xidl/commit/43ecad3abf01d594fc0a51a9f2aec0779522c029))
* **axum:** impl auth for client ([9612ce5](https://github.com/xidl/xidl/commit/9612ce5cf4f37627e2c3bc4228d5c8863df2878e))
* **axum:** impl http_bearer ([275c165](https://github.com/xidl/xidl/commit/275c165bb043091c06fb89dbd6c50a0f8aa276d6))
* **axum:** render reqwest error by debug ([6b6908f](https://github.com/xidl/xidl/commit/6b6908f4ed0a344095830bf7ebb9515fd4fe59aa))
* **axum:** support pluggable http body codecs ([f2a8cdd](https://github.com/xidl/xidl/commit/f2a8cdd824cf4ae89fab8b66aa9f9db80e900878))
* **axum:** update attribute generate ([1961984](https://github.com/xidl/xidl/commit/1961984053cb454378057d50fab5358ece270081))
* **bitmask:** support bitbound ([270ca60](https://github.com/xidl/xidl/commit/270ca602fe426a595f83cd7adcb5b8da88c0a2a1))
* complete http stream and jsonrpc stream ([41373f6](https://github.com/xidl/xidl/commit/41373f6d6fa2e6cb78b4590bfcdc6e45202f1099))
* **docs:** translate REST and RFC documentation to English ([10b3187](https://github.com/xidl/xidl/commit/10b3187a8727c915d5fd31dfea19f672ed813267))
* **docs:** update docs ([e954096](https://github.com/xidl/xidl/commit/e9540961b653c18fe56de3fd2a359337c8f52e3c))
* **gen:** change default openapi filename to openapi_{filename}.json ([10c95c3](https://github.com/xidl/xidl/commit/10c95c30eae3e75a3d28e36d5a0f4ddeb7fb2895))
* **git:** add Cargo.lock ([87b3497](https://github.com/xidl/xidl/commit/87b3497fed31d92f57b0b1b603f75f87333392bd))
* **go-http:** complete HTTP RFC support ([f9f7896](https://github.com/xidl/xidl/commit/f9f7896f63ae92d4cbfb6f0da96cec8f9c0f2aec))
* **go:** add support for [@cors](https://github.com/cors) annotation ([253ce87](https://github.com/xidl/xidl/commit/253ce874cd2aed65f2362e4d1c4fb6d0c0cd8d0e))
* **go:** auto-flatten single composite param or return value ([a5b5161](https://github.com/xidl/xidl/commit/a5b5161df97a46108ce15f050ace46db50a8854b))
* **golang:** add xidl-go-json reflection-based library ([c8df2b2](https://github.com/xidl/xidl/commit/c8df2b2d66eb96e0490a45a750ba07b3e6b36f5c))
* **golang:** support catch-all map and any flatten fields in xidl-go-json ([1fa0619](https://github.com/xidl/xidl/commit/1fa0619831475deb8bd6cabbeafe4af7477e3f72))
* **golang:** support flatten tag in xidl-go-json ([d8f428d](https://github.com/xidl/xidl/commit/d8f428d3a5ab45673fd4c68527341a6fbdafa7dc))
* **golang:** use xidl-go-json codec in go-rest and emit xjson tags in xidlc ([772f7ff](https://github.com/xidl/xidl/commit/772f7ffb1916975d5d896fd36ca0c033cb51a2d4))
* **go:** migrate go-rest generator from net/http to gin ([d682fc9](https://github.com/xidl/xidl/commit/d682fc9615d43f42a56e3abb8e5b9368bf487b5b))
* **hir:** flatten constexpr ([79934b4](https://github.com/xidl/xidl/commit/79934b4d3d2be3d7d92df3f0c7512c7638211e1b))
* **http:** add [@header](https://github.com/header) support ([d01783a](https://github.com/xidl/xidl/commit/d01783a207c5a90cc2ea442670925e85d3755c5e))
* **http:** add body and flatten annotations ([8753d92](https://github.com/xidl/xidl/commit/8753d929c5ad9829e796700b39d781b1689f88ce))
* **http:** implement raw text serialization for primitive types and update BDD tests ([a72654f](https://github.com/xidl/xidl/commit/a72654f59651c7866e78ee12e7b09fb52497cd5b))
* **http:** standardize error response format to {code, msg} and add BDD bad path tests ([a7cc8d3](https://github.com/xidl/xidl/commit/a7cc8d320968e77a05cdaf8de304264ac423b1ca))
* **jsonpc:** add tcp and inproc support ([e6c7b93](https://github.com/xidl/xidl/commit/e6c7b93a32f5aaed976d38622ae402c202fd5352))
* **jsonrpc:** add ipc transport for plugins ([5d9cfdb](https://github.com/xidl/xidl/commit/5d9cfdbf026474594f71303ab2a8c22a5ab9b9d0))
* **jsonrpc:** add quic support ([c07e660](https://github.com/xidl/xidl/commit/c07e6605b2cc533eee5005345650048217fc08cb))
* **jsonrpc:** add ws, wss, tls support ([fc98fdc](https://github.com/xidl/xidl/commit/fc98fdc7781bdff5f3f9dfeabe61764457109096))
* **jsonrpc:** expose bound server endpoint ([a933984](https://github.com/xidl/xidl/commit/a9339842cf72655d2670dc6a5eafcd4ad95840e5))
* **jsonrpc:** impl stream for jsonrpc ([df85fe5](https://github.com/xidl/xidl/commit/df85fe5ee9b241047eb265352eb4162cbe5bb6fa))
* **jsonrpc:** unify client transport around stream ([4eecedc](https://github.com/xidl/xidl/commit/4eecedc57ae839fc6b94b11c0a9c169ed603864d))
* **jsonrpc:** update rust_jsonrpc ([66f9448](https://github.com/xidl/xidl/commit/66f9448bb73261edef8bc487a78f9cf9fb6b5d0c))
* **keycloak:** update keycloak metadata ([795e85d](https://github.com/xidl/xidl/commit/795e85d5a21857da6b7e519d8826258348e29618))
* make build faster ([908a6d7](https://github.com/xidl/xidl/commit/908a6d74bacdd7614a8fbea1551fad6a54163e84))
* **openapi:** auto select openapi version ([6aad440](https://github.com/xidl/xidl/commit/6aad4400f232db1e2017983e7ccdb07015673e30))
* **openapi:** support progma xidlc service ([649f542](https://github.com/xidl/xidl/commit/649f542540940b2ccf013e72c2613243ce54f87c))
* **playground:** add openapi and openrpc ([e68971d](https://github.com/xidl/xidl/commit/e68971d23779c0beafa29ce2a7356050a14d5ec6))
* **pre-commit:** support pre-commit ([a82ca67](https://github.com/xidl/xidl/commit/a82ca6700b7b338133e126869629f12c033f5ae3))
* **release:** mirror root releases to Go module tags ([7c8c588](https://github.com/xidl/xidl/commit/7c8c5884c5754bf1020c0118dc91ac41fbe6f565))
* remove playground ([bae5e45](https://github.com/xidl/xidl/commit/bae5e451a4947b22ca585a6f5d850248bc048774))
* rename *http to *rest ([a9fe5dd](https://github.com/xidl/xidl/commit/a9fe5dd13e426183ee5f8f061d2bb958b18fe91f))
* **rename:** rename typescript-json to typescript-codec and go-json to go-codec ([adb9880](https://github.com/xidl/xidl/commit/adb988056ddfd81472d2272c0c9f4893538ce719))
* reorganization features ([e462694](https://github.com/xidl/xidl/commit/e4626946b3f7821b39381baf6edf21454598be08))
* **rest-hir:** implement enriched HIR mapping and migrate all generators ([1919bf5](https://github.com/xidl/xidl/commit/1919bf50d18ae23bd6ba72ce51b7d04f75beb2cc))
* **rust-axum:** add support for text/plain mime type ([0fdd2d4](https://github.com/xidl/xidl/commit/0fdd2d480da074d7894476bf4a7b3cb93d991b77))
* **rust-axum:** impl bidi_stream ([9b1f1b8](https://github.com/xidl/xidl/commit/9b1f1b883af35016161d33bcd59753a0d319a701))
* **rust-axum:** implement reachability analysis and conditional generation ([d22c08d](https://github.com/xidl/xidl/commit/d22c08dfe10c5455a13fa92726339a79bba938e9))
* **rust-gen:** add --mock flag to generate mockall traits ([bb04fea](https://github.com/xidl/xidl/commit/bb04feaf76858a42371ee52f333416894373d39e))
* **rust-gen:** add support for recursive union types ([cbd0148](https://github.com/xidl/xidl/commit/cbd01481511d380996069fa6a3f1abcb6345797b))
* **rust-jsonrpc:** add conditional compilation for unsupported platforms ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **rust-jsonrpc:** optimize bidirectional method matching ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **skills:** add code-style validation skill ([9422b40](https://github.com/xidl/xidl/commit/9422b407d316dfa381ca4314fb7b36105fdde7f6))
* standardize Default trait and improve annotation support for all Rust types ([491d57e](https://github.com/xidl/xidl/commit/491d57e6d99317a492fb2f786ad12847093f93b5))
* **stream:** using writer on the client_side ([9dc3d1b](https://github.com/xidl/xidl/commit/9dc3d1bd8c53a76194f5ed19c7ff562c1dd18b9e))
* support [@name](https://github.com/name) on enum members ([909595f](https://github.com/xidl/xidl/commit/909595fae6b450a2b12d25ef74977374d0f14c2d))
* **ts:** generate interface for ts ([0e0ae3a](https://github.com/xidl/xidl/commit/0e0ae3a63924dc06b6698452e4bcd38edb398567))
* **typescript:** add typescript-xidl-json library for metadata-driven serialization ([48f0327](https://github.com/xidl/xidl/commit/48f0327804fea46d11a80d16a9dd7c2388a10dbf))
* **typescript:** migrate to xidl-typescript-codec for metadata-aware serialization ([#178](https://github.com/xidl/xidl/issues/178)) ([3c7b082](https://github.com/xidl/xidl/commit/3c7b082af74cd2ef296e0ab75d814b070711498a))
* **typescript:** rename package to xidl-typescript-json ([9ed12ff](https://github.com/xidl/xidl/commit/9ed12ff89de3e1a379d7d42a417df447a8a6428e))
* unify golang versions in release-please and remove xidlc-examples from tracking ([d900fef](https://github.com/xidl/xidl/commit/d900fef239dd22c3ca78cee6a9121dee4bdb1200))
* using underscore instead of - in annotation ([01e53bf](https://github.com/xidl/xidl/commit/01e53bf5e2310bd26e49aea6ab53c2b88f175104))
* **website:** add Google Analytics integration via environment variable ([f850c8e](https://github.com/xidl/xidl/commit/f850c8e9029bbef6c7b82e291f65dcb6dc765ed2))
* **website:** support highlight idl ([9d3667e](https://github.com/xidl/xidl/commit/9d3667e9709330c429f72417492cff000a517ff3))
* **website:** update docs ([0400c26](https://github.com/xidl/xidl/commit/0400c267ca1ec9891cdc99a89b603a76bd6d97e0))
* **workflow:** add dynamic release context resolution and manual dispatch support ([f9d0d24](https://github.com/xidl/xidl/commit/f9d0d24a02e5f4a0da40c6731673472673caa5b2))
* **xidl-api-keycloak:** build client ([8d95c39](https://github.com/xidl/xidl/commit/8d95c3921113ca079d20cbed9cad11f9915caa5d))
* **xidl-build:** allow set openapi and openrpc output file name ([59eb939](https://github.com/xidl/xidl/commit/59eb939598f1f1693d2658d50a6c22f1c9ba4aba))
* **xidl-build:** expose more option ([aeb035b](https://github.com/xidl/xidl/commit/aeb035b13d8fec565cd3ce6d3adf75f5df410439))
* **xidl-go-codex:** compatible with standard json ([89b6245](https://github.com/xidl/xidl/commit/89b6245387990c7ee5070bec028aa2afbf843682))
* **xidl-go-json:** add more complex flatten rule ([0c0385b](https://github.com/xidl/xidl/commit/0c0385beca9b4ff9555b17ddd982492285f92d92))
* **xidl-parser, xidlc:** add rename/serialize_name/deserialize_name/rename_all annotations ([5827409](https://github.com/xidl/xidl/commit/5827409a098831c0004841140c60a90b9bd7d8d3))
* **xidl-parser:** add recursive type semantic analysis ([905dc4a](https://github.com/xidl/xidl/commit/905dc4a4ec90ae9dd008c8f34f8ee2665aee6219))
* **xidl-parser:** parse hir IntegerSign and IntegerBoolean ([1029226](https://github.com/xidl/xidl/commit/10292266c6593cea3fbc2f562f11de02226f91e0))
* **xidl-parser:** parse IntegerLiteral ([e744d1a](https://github.com/xidl/xidl/commit/e744d1aedaa69cb6b9cc8ec97551132071025ea3))
* **xidl-rust-axum:** make reqwest as a optional dep ([743d5ad](https://github.com/xidl/xidl/commit/743d5ad917952d9b9a9c3ad9d3bdef115db0dc20))
* **xidl-rust-axum:** update error model ([5712624](https://github.com/xidl/xidl/commit/5712624aa0e8046affd7e39aeacf6775b3e9ea2d))
* **xidlc:** add go and go-http targets ([2c4d4c4](https://github.com/xidl/xidl/commit/2c4d4c4a28076127334da203d5d62ccd14553e64))
* **xidlc:** add openrpc support ([081a4a9](https://github.com/xidl/xidl/commit/081a4a9bd8c2430b3e502db792705593d8fe4142))
* **xidlc:** add python http generators and runtime ([1cdff5a](https://github.com/xidl/xidl/commit/1cdff5a0a8f1f9ce4d7dfda9cead47396a06efb4))
* **xidlc:** add skip cdr codec flag ([8174ae2](https://github.com/xidl/xidl/commit/8174ae2889f23aeb8006d7dd96f8213bbdc8e568))
* **xidlc:** add typescript http server generation ([1781c4a](https://github.com/xidl/xidl/commit/1781c4a751694d7190b52b541f0a62ca64b553cd))
* **xidlc:** clean rust axum warning ([4222f65](https://github.com/xidl/xidl/commit/4222f65e68277b7c097f85a326daac72ce078484))
* **xidlc:** clean warning ([0da710a](https://github.com/xidl/xidl/commit/0da710a81317d923c8eb7fd4d006fd283c5b48bd))
* **xidlc:** decouple axum unary request and response transport ([a1ed93b](https://github.com/xidl/xidl/commit/a1ed93b0a56b85d2aff5fc47b06c60338bd141dc))
* **xidlc:** don't expand interface when generate openapi ([a52c87c](https://github.com/xidl/xidl/commit/a52c87ce2a8ef394ab52f30d075d2030e4102617))
* **xidlc:** dont't generate ts and openapi in axum ([b86dd13](https://github.com/xidl/xidl/commit/b86dd131dbc54fdff55eaf1680385d480d1b3de0))
* **xidlc:** generate service code by default ([d03ccf6](https://github.com/xidl/xidl/commit/d03ccf6f00467ccba89be6d5bf6645d7f73424c3))
* **xidlc:** generate typed authentication parameters and conditional constructors for Rust Axum ([3aae8f6](https://github.com/xidl/xidl/commit/3aae8f6f1f5269d4e26203c81fc06687ef6c86e7))
* **xidlc:** impl cors annotation ([7f6ed42](https://github.com/xidl/xidl/commit/7f6ed42a4b6e0577120c23b9d9f85aac0ddc7287))
* **xidlc:** impl default for rust enum ([f9868ac](https://github.com/xidl/xidl/commit/f9868ac41fdd0644d051a02d597bdea0d7fc017f))
* **xidlc:** impl default for rust struct ([24d996f](https://github.com/xidl/xidl/commit/24d996f872d26203069a8a11be56b22d1f5c90f5))
* **xidlc:** impl stream for axum, ts and openapi ([64e8344](https://github.com/xidl/xidl/commit/64e834425ae2fb6df650d333a840b848d1879f12))
* **xidlc:** implement rename/rename_all annotations for golang ([211a128](https://github.com/xidl/xidl/commit/211a1284c5497f016d1798d28040fd2173343dc9))
* **xidlc:** make fmt as a feature ([4ff313a](https://github.com/xidl/xidl/commit/4ff313a0ec01ce29f0a1a5b9d9a1b71c2e107345))
* **xidlc:** remove c and cpp codegen support ([02bdecb](https://github.com/xidl/xidl/commit/02bdecbd9b648cbd20e90cc2c1c0391cb7710616))
* **xidlc:** remove uncessary attribute ([bcdb9b6](https://github.com/xidl/xidl/commit/bcdb9b65c756fc5cd97f9e8cde18c063821678d6))
* **xidlc:** render constructor by service ([a476311](https://github.com/xidl/xidl/commit/a47631193f39fb52c3b5eb556d3e3a3af0f90bda))
* **xidlc:** render doc ([c4d32e7](https://github.com/xidl/xidl/commit/c4d32e7b69803108c192496a768b3315b20f91af))
* **xidlc:** render doc ([eab28b5](https://github.com/xidl/xidl/commit/eab28b5968b17b122a4a4b6277e79a16f59a25f4))
* **xidlc:** replace rust formatter with prettyplease ([87e1b26](https://github.com/xidl/xidl/commit/87e1b269db59f60a1df38e30a3bd7900dee27273))
* **xidlc:** split typescript http generation ([793ea44](https://github.com/xidl/xidl/commit/793ea44d646b45b18a4b244c3845b6c88a4cda15))
* **xidlc:** support [@rust](https://github.com/rust) annotation ([a8f10d6](https://github.com/xidl/xidl/commit/a8f10d64b2ad6770543c7ea96c37a8086f280bfd))
* **xidlc:** support HIR include expansion ([42a715b](https://github.com/xidl/xidl/commit/42a715bfe6a2b7021f8392aca4e352a69efbb6ef))
* **xidlc:** update format ([0695679](https://github.com/xidl/xidl/commit/0695679d823da801e0473c465b586d026701f21f))
* **xidlc:** update jinja formatter ([41bf57a](https://github.com/xidl/xidl/commit/41bf57af999f5d398528d7a0d0aa9252b6ea8ea5))
* **xidlc:** update REST generators and snapshots to support cross-language BDD requirements ([d8e59c3](https://github.com/xidl/xidl/commit/d8e59c314b03cb7273a378dd0045ee7343ebe672))
* **xidl:** remove timestampe in header ([c04c585](https://github.com/xidl/xidl/commit/c04c585b3a7381282729708583cefacd6f000990))
* **xidl:** set lints.workspace=true ([2e63ba4](https://github.com/xidl/xidl/commit/2e63ba408b6a0d84887f3a8f09e887c13e39cadb))
* **xildc-jsonrpc:** rename feature ([b5e3f9a](https://github.com/xidl/xidl/commit/b5e3f9a503153c4d0105bd6cdd7360f61f9236e8))


### Bug Fixes

* address CI failures by applying formatting and updating Go snapshots ([7b8355b](https://github.com/xidl/xidl/commit/7b8355bf43d807e4d2f46c059dfd17aa5a194862))
* **bdd:** avoid duplicate TS server listeners ([9b0ab5b](https://github.com/xidl/xidl/commit/9b0ab5b95afcde26a419e1008322d302838ab0d3))
* **bdd:** avoid ephemeral port reuse ([30c9a6b](https://github.com/xidl/xidl/commit/30c9a6b00cf2e61564625c9e583f789f5f5229bb))
* **bdd:** clean up server process groups ([7669e99](https://github.com/xidl/xidl/commit/7669e9980cdf625c5e54536f03f94c07ca770623))
* **bdd:** extend rust boilerplate startup wait ([ba1a1de](https://github.com/xidl/xidl/commit/ba1a1de9af67c8565faad7769b0d47bfb9d85cf2))
* **bdd:** reserve ports before starting test servers ([991d933](https://github.com/xidl/xidl/commit/991d933d56574ca4cd429a1ea966717bbb499d6a))
* enable subpath tags for Go modules and update internal dependencies using release-please ([4a5591b](https://github.com/xidl/xidl/commit/4a5591b1f969c5ae73bf48821d3cb2e06193c073))
* fix build on docs.rs ([f591ba3](https://github.com/xidl/xidl/commit/f591ba3f856c2d7be49209617c055929bf7145b4))
* fix build on docs.rs ([44e1efb](https://github.com/xidl/xidl/commit/44e1efb499464085510561657adbe6c25d94dbe7))
* fix cargo publish ([60f576e](https://github.com/xidl/xidl/commit/60f576eae160676e2e2b25306de5c2708da4a968))
* fix err code ([4c66e24](https://github.com/xidl/xidl/commit/4c66e2427e833ab1211d800242390f43ab017ac0))
* fix golang tag ([67737bc](https://github.com/xidl/xidl/commit/67737bc8cf945f5480de3ea77ed06abf42bc649f))
* fix release problem ([c35f9ac](https://github.com/xidl/xidl/commit/c35f9ac0c3fff69d297d2f0508663e8736e07e97))
* fix warning ([f29196c](https://github.com/xidl/xidl/commit/f29196c9eecce22b1c4019e64f25a8e16740e994))
* **formula:** update xidlc.rb SHA256 checksum for v0.32.0 ([3b3c71c](https://github.com/xidl/xidl/commit/3b3c71c05a53b1875b03f0f115a5baebf4cda0df))
* **generator:** resolve clippy warnings for redundant field names in Rust Axum from_request implementation and unused variable in Go template ([4b8f1c3](https://github.com/xidl/xidl/commit/4b8f1c30131b97269b45163fa53853ed0df2e32b))
* **go-http:** correct path parameter pattern replacement logic ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **go-rest:** honor client and server generation flags ([f99999f](https://github.com/xidl/xidl/commit/f99999f2e03da78160d277c38f6d3f21276e0234))
* **go:** add delegated go test targets and ci coverage ([38acb67](https://github.com/xidl/xidl/commit/38acb671537301330bb3199eb6dfed22fc180305))
* **go:** preserve acronyms in type and field names ([d68d9a1](https://github.com/xidl/xidl/commit/d68d9a1e1db7dfde95fe26fbe866bcff58303b34))
* **go:** update GinWriteJSONError calls in examples to match new signature ([dca7e75](https://github.com/xidl/xidl/commit/dca7e758a1502969dafdbe50d94e27fb47342d62))
* **http:** support comma-separated cors annotation params ([c631952](https://github.com/xidl/xidl/commit/c63195210e62db505d4720b69f1d49778c217c5f))
* **http:** wrap void out responses as objects ([b5ef38d](https://github.com/xidl/xidl/commit/b5ef38d454d941c18c98453bf5be492459c77922))
* **idl:** normalize oauth scopes for fmt and pre-commit ([76e3fbc](https://github.com/xidl/xidl/commit/76e3fbcc41cd5762d2beaff70399f355536146b8))
* **jsonrpc:** fix jsonrpc ([ffa6aac](https://github.com/xidl/xidl/commit/ffa6aacf5df26ccd1d39f72f1a38d250178f8220))
* **jsonrpc:** generate arc-backed service servers ([0f1a2ca](https://github.com/xidl/xidl/commit/0f1a2ca65469aa944f9fd297d067e55644df0dd3))
* **openrpc:** fix openrpc generate ([359c228](https://github.com/xidl/xidl/commit/359c2289dac54ae9637f176d031c8cca20fdf7a4))
* **parser:** preserve unknown pragmas in hir ([fffff6e](https://github.com/xidl/xidl/commit/fffff6ed4e2711f21f3d81ba9249cd9482ae185a))
* **python:** fix FastAPI adapter path parameter handling and unit tests ([dd9a562](https://github.com/xidl/xidl/commit/dd9a562d549355255cd985f6da50ab65127aed17))
* **python:** install runtime test dependencies ([60446dc](https://github.com/xidl/xidl/commit/60446dc9cf21d91d5572179ebd58dbc3f3cd0208))
* **python:** repair runtime test target ([75ae2ce](https://github.com/xidl/xidl/commit/75ae2ced7f85e6d7504678f05f25bba235212ff9))
* **python:** resolve FastAPI 422 error by overriding endpoint signature ([b075934](https://github.com/xidl/xidl/commit/b075934b39cfec31d0b38c265011bd147351e470))
* remove gen-hir and gen-typed-ast feature ([c8a14d7](https://github.com/xidl/xidl/commit/c8a14d7719beb3a930a8c4ddf09a3e90d9ce2d31))
* **rest:** ensure SingleValue body shapes for text/plain are not wrapped in a JSON object in TS, Rust, Go, and Python generators ([208e697](https://github.com/xidl/xidl/commit/208e697c29e963b4b13233a69200bd909dda840c))
* **rest:** ensure SingleValue response bodies for text/plain are mapped to type aliases instead of structs in Rust Axum generator and fix test assertions ([cf1924b](https://github.com/xidl/xidl/commit/cf1924bb43fa73e41a9e09a403a90391be134275))
* **rest:** skip strict Accept header validation unless explicitly configured ([2b16ee7](https://github.com/xidl/xidl/commit/2b16ee707423f2510d8a517b033a5daeb61ed353))
* **rust-axum:** fix template ([bac9a85](https://github.com/xidl/xidl/commit/bac9a85412b81c28e480edc70138dcc7a02d74b4))
* **rust:** qualify generated BTreeMap paths ([6ff8293](https://github.com/xidl/xidl/commit/6ff8293950da728bf5801e87ac2dce028cc2a861))
* **ts:** resolve BDD test failures and improve REST generator stability ([6c3dd99](https://github.com/xidl/xidl/commit/6c3dd99305aae69d978937ef80ad94f22ecd399c))
* **typeobject:** restore generation and compilation for typeobject idl ([6bc389a](https://github.com/xidl/xidl/commit/6bc389aba4797b97f3f0a9d7576ac82696f0cb73))
* typo ([92ed86b](https://github.com/xidl/xidl/commit/92ed86b742fc9840337d0d75a4e5a9c7b94e9ed0))
* **website:** fix twcss support ([7aff46d](https://github.com/xidl/xidl/commit/7aff46d137adb5f24bf1f3202e419c682c4c45b4))
* **workflow:** enhance release metadata resolution with version parsing ([3b3c71c](https://github.com/xidl/xidl/commit/3b3c71c05a53b1875b03f0f115a5baebf4cda0df))
* **workflow:** ensure publish-release workflow runs on all events ([fd99ec7](https://github.com/xidl/xidl/commit/fd99ec79216bf49b31f6554adff2f7a0dd6a99f7))
* **xidl-parser:** support positional path in HTTP verb annotations ([45f1356](https://github.com/xidl/xidl/commit/45f13560f1615ceb8ca47e73c74ef58de013c16a))
* **xidlc-examples:** address clippy warning in rest_snapshots test ([07b40c5](https://github.com/xidl/xidl/commit/07b40c548d4217266a2d7de5d4df68356475e2ae))
* **xidlc:** add default impls for rust unions ([4b19bea](https://github.com/xidl/xidl/commit/4b19beaf343e7cd203431be8d36f671aaeb5b1ab))
* **xidlc:** add version and git hash to generated headers ([dbfce97](https://github.com/xidl/xidl/commit/dbfce977bf363d6c7c17d4a2b1c10a07234ebdca))
* **xidlc:** address clippy warnings in generator and importer ([bb5bb01](https://github.com/xidl/xidl/commit/bb5bb01e099bbf6c5a64745dab12c1e49c84e9ed))
* **xidlc:** correct jinja formatter block indentation ([944ed1f](https://github.com/xidl/xidl/commit/944ed1f6fe1545b415a294da696a3503b0cf4bcc))
* **xidlc:** fix axum generate ([da8c841](https://github.com/xidl/xidl/commit/da8c841e8350e4705b68a5a08c8916a689e7e69b))
* **xidlc:** fix return value with sequence ([0173ef3](https://github.com/xidl/xidl/commit/0173ef39f6b0dd72108c3bf048cf27db495f2e44))
* **xidlc:** fix wasm build ([782df53](https://github.com/xidl/xidl/commit/782df53b8ba70cc40e08b0dcbf534b29a506e162))
* **xidlc:** generate pointer types and correct bindings for optional Go REST parameters ([a40c8b0](https://github.com/xidl/xidl/commit/a40c8b058f74308b88aaab78fbc6069165d77efc))
* **xidlc:** import referenced model schemas in iface.zod.ts (close [#172](https://github.com/xidl/xidl/issues/172)) ([6b1caa0](https://github.com/xidl/xidl/commit/6b1caa06054032d1ed2e38c76a50d49b1a5d7592))
* **xidlc:** map rust any to serde_json value ([7376020](https://github.com/xidl/xidl/commit/7376020ec717ccc6bd33588d5d27bee76e470592))
* **xidlc:** move rust-axum transport rendering into template context ([d31969c](https://github.com/xidl/xidl/commit/d31969c8adc568d1f4dce41aa3b6ca7aa0564304))
* **xidlc:** resync ipc jsonrpc bindings ([3287c39](https://github.com/xidl/xidl/commit/3287c39e59bc739d06290d1bb3f6920f735e6617))
* **xidlc:** serialize rust unions with real string tags ([a6cb9e9](https://github.com/xidl/xidl/commit/a6cb9e9fd57c19d8508d0ea9a5d417ee450bc182))
* **xidlc:** support standalone JSON Schema and  in openapi import ([6ea0946](https://github.com/xidl/xidl/commit/6ea0946b975371611fdae9873ba88bd1b04271b1))
* **xidlc:** use item-level allow attrs in rust output ([906149e](https://github.com/xidl/xidl/commit/906149eb4c65d8e28dbdad4922e3831f0d7235e2))
* **xidl:** fix multi interface in same file ([02fe850](https://github.com/xidl/xidl/commit/02fe8503e0822bcfa2b298da641e3d6810317753))
* **xidl:** fix package name ([a406176](https://github.com/xidl/xidl/commit/a4061764baee5212eba56b0117e1472998d666af))

## [0.81.0](https://github.com/xidl/xidl/compare/v0.80.1...v0.81.0) (2026-06-22)


### Features

* add [@cookie](https://github.com/cookie) support for http generators ([50c005b](https://github.com/xidl/xidl/commit/50c005b073f425a0fb24e5ec6d767445cac0443b))
* add [@http](https://github.com/http)(rename) for field serialization ([d7cb3c8](https://github.com/xidl/xidl/commit/d7cb3c88783f0b115d585a4fd7acbb8120b21697))
* add [@name](https://github.com/name) field rename ([bef6497](https://github.com/xidl/xidl/commit/bef64973f20eae1f42eab971757a2049ba0945ec))
* add [@skip](https://github.com/skip) annotation to support skipping fields during serialization ([7cbf6c8](https://github.com/xidl/xidl/commit/7cbf6c80365acf4f9f50c89e6547d973f8c2a6c9))
* add CD workflow and improve release automation ([755d4be](https://github.com/xidl/xidl/commit/755d4be6068188b2763e6976643a61282d1a7d90))
* add default Rust derives for struct and enum ([b25434b](https://github.com/xidl/xidl/commit/b25434bb9006bad056eabe917801506980521b3f))
* add doc comment to annotation ([bb1e191](https://github.com/xidl/xidl/commit/bb1e1918d14cc56657777bdf598a031cce209af6))
* add Homebrew formula and improve Windows package support ([4ad1161](https://github.com/xidl/xidl/commit/4ad116103c09680e680b1ecaca848cec4e88f477))
* add http hir layer ([0f3a307](https://github.com/xidl/xidl/commit/0f3a307537f96b9d0b9e56739098805c7c45f7fe))
* add hy2 ([b1c0100](https://github.com/xidl/xidl/commit/b1c0100b80d7155804a3001a4e4d53dcd7d6b04b))
* add keycloak ([47ca2e3](https://github.com/xidl/xidl/commit/47ca2e3f0752c834f021a113f92c6172b4e55555))
* add msgpack support ([bc5876c](https://github.com/xidl/xidl/commit/bc5876c5508844b06b5c6980161fa0e0c9a43051))
* Add OpenAPI 3.2 HTTP stream support ([7d285d6](https://github.com/xidl/xidl/commit/7d285d69b853102c66021cdf96a2a8346fe3f592))
* add ugprade ([ea48027](https://github.com/xidl/xidl/commit/ea48027f36a34d1befe8f82c8666b07bfbfa35f4))
* add upgrade annotation support ([b534408](https://github.com/xidl/xidl/commit/b5344084ccc9e4680c97a240b914b31330ee04cf))
* add xidl-api-discord ([c4cb195](https://github.com/xidl/xidl/commit/c4cb195486619caf54d21f73277934394ff93916))
* add xidl-api-github and openapi importer ([d0475ba](https://github.com/xidl/xidl/commit/d0475bae7ef5a5bfa1eaee26fd749647ef6a18b4))
* add xidl-apis-reddit ([5274a9c](https://github.com/xidl/xidl/commit/5274a9cf1c506c8f249e0d759f4fa7423424750f))
* align unary http and http security mapping ([c0dad07](https://github.com/xidl/xidl/commit/c0dad0728aa735267ecc3715d4ef04871588c015))
* apply [@name](https://github.com/name) to rust struct fields ([ebe42f0](https://github.com/xidl/xidl/commit/ebe42f0efb5ec2a5a69a8576738a0ef66218ba43))
* **axu,:** impl basic-auth ([43ecad3](https://github.com/xidl/xidl/commit/43ecad3abf01d594fc0a51a9f2aec0779522c029))
* **axum:** impl auth for client ([9612ce5](https://github.com/xidl/xidl/commit/9612ce5cf4f37627e2c3bc4228d5c8863df2878e))
* **axum:** impl http_bearer ([275c165](https://github.com/xidl/xidl/commit/275c165bb043091c06fb89dbd6c50a0f8aa276d6))
* **axum:** render reqwest error by debug ([6b6908f](https://github.com/xidl/xidl/commit/6b6908f4ed0a344095830bf7ebb9515fd4fe59aa))
* **axum:** support pluggable http body codecs ([f2a8cdd](https://github.com/xidl/xidl/commit/f2a8cdd824cf4ae89fab8b66aa9f9db80e900878))
* **axum:** update attribute generate ([1961984](https://github.com/xidl/xidl/commit/1961984053cb454378057d50fab5358ece270081))
* **bitmask:** support bitbound ([270ca60](https://github.com/xidl/xidl/commit/270ca602fe426a595f83cd7adcb5b8da88c0a2a1))
* complete http stream and jsonrpc stream ([41373f6](https://github.com/xidl/xidl/commit/41373f6d6fa2e6cb78b4590bfcdc6e45202f1099))
* **docs:** translate REST and RFC documentation to English ([10b3187](https://github.com/xidl/xidl/commit/10b3187a8727c915d5fd31dfea19f672ed813267))
* **docs:** update docs ([e954096](https://github.com/xidl/xidl/commit/e9540961b653c18fe56de3fd2a359337c8f52e3c))
* **gen:** change default openapi filename to openapi_{filename}.json ([10c95c3](https://github.com/xidl/xidl/commit/10c95c30eae3e75a3d28e36d5a0f4ddeb7fb2895))
* **git:** add Cargo.lock ([87b3497](https://github.com/xidl/xidl/commit/87b3497fed31d92f57b0b1b603f75f87333392bd))
* **go-http:** complete HTTP RFC support ([f9f7896](https://github.com/xidl/xidl/commit/f9f7896f63ae92d4cbfb6f0da96cec8f9c0f2aec))
* **go:** add support for [@cors](https://github.com/cors) annotation ([253ce87](https://github.com/xidl/xidl/commit/253ce874cd2aed65f2362e4d1c4fb6d0c0cd8d0e))
* **go:** auto-flatten single composite param or return value ([a5b5161](https://github.com/xidl/xidl/commit/a5b5161df97a46108ce15f050ace46db50a8854b))
* **golang:** add xidl-go-json reflection-based library ([c8df2b2](https://github.com/xidl/xidl/commit/c8df2b2d66eb96e0490a45a750ba07b3e6b36f5c))
* **golang:** support catch-all map and any flatten fields in xidl-go-json ([1fa0619](https://github.com/xidl/xidl/commit/1fa0619831475deb8bd6cabbeafe4af7477e3f72))
* **golang:** support flatten tag in xidl-go-json ([d8f428d](https://github.com/xidl/xidl/commit/d8f428d3a5ab45673fd4c68527341a6fbdafa7dc))
* **golang:** use xidl-go-json codec in go-rest and emit xjson tags in xidlc ([772f7ff](https://github.com/xidl/xidl/commit/772f7ffb1916975d5d896fd36ca0c033cb51a2d4))
* **go:** migrate go-rest generator from net/http to gin ([d682fc9](https://github.com/xidl/xidl/commit/d682fc9615d43f42a56e3abb8e5b9368bf487b5b))
* **hir:** flatten constexpr ([79934b4](https://github.com/xidl/xidl/commit/79934b4d3d2be3d7d92df3f0c7512c7638211e1b))
* **http:** add [@header](https://github.com/header) support ([d01783a](https://github.com/xidl/xidl/commit/d01783a207c5a90cc2ea442670925e85d3755c5e))
* **http:** add body and flatten annotations ([8753d92](https://github.com/xidl/xidl/commit/8753d929c5ad9829e796700b39d781b1689f88ce))
* **http:** implement raw text serialization for primitive types and update BDD tests ([a72654f](https://github.com/xidl/xidl/commit/a72654f59651c7866e78ee12e7b09fb52497cd5b))
* **http:** standardize error response format to {code, msg} and add BDD bad path tests ([a7cc8d3](https://github.com/xidl/xidl/commit/a7cc8d320968e77a05cdaf8de304264ac423b1ca))
* **jsonpc:** add tcp and inproc support ([e6c7b93](https://github.com/xidl/xidl/commit/e6c7b93a32f5aaed976d38622ae402c202fd5352))
* **jsonrpc:** add ipc transport for plugins ([5d9cfdb](https://github.com/xidl/xidl/commit/5d9cfdbf026474594f71303ab2a8c22a5ab9b9d0))
* **jsonrpc:** add quic support ([c07e660](https://github.com/xidl/xidl/commit/c07e6605b2cc533eee5005345650048217fc08cb))
* **jsonrpc:** add ws, wss, tls support ([fc98fdc](https://github.com/xidl/xidl/commit/fc98fdc7781bdff5f3f9dfeabe61764457109096))
* **jsonrpc:** expose bound server endpoint ([a933984](https://github.com/xidl/xidl/commit/a9339842cf72655d2670dc6a5eafcd4ad95840e5))
* **jsonrpc:** impl stream for jsonrpc ([df85fe5](https://github.com/xidl/xidl/commit/df85fe5ee9b241047eb265352eb4162cbe5bb6fa))
* **jsonrpc:** unify client transport around stream ([4eecedc](https://github.com/xidl/xidl/commit/4eecedc57ae839fc6b94b11c0a9c169ed603864d))
* **jsonrpc:** update rust_jsonrpc ([66f9448](https://github.com/xidl/xidl/commit/66f9448bb73261edef8bc487a78f9cf9fb6b5d0c))
* **keycloak:** update keycloak metadata ([795e85d](https://github.com/xidl/xidl/commit/795e85d5a21857da6b7e519d8826258348e29618))
* make build faster ([908a6d7](https://github.com/xidl/xidl/commit/908a6d74bacdd7614a8fbea1551fad6a54163e84))
* **openapi:** auto select openapi version ([6aad440](https://github.com/xidl/xidl/commit/6aad4400f232db1e2017983e7ccdb07015673e30))
* **openapi:** support progma xidlc service ([649f542](https://github.com/xidl/xidl/commit/649f542540940b2ccf013e72c2613243ce54f87c))
* **playground:** add openapi and openrpc ([e68971d](https://github.com/xidl/xidl/commit/e68971d23779c0beafa29ce2a7356050a14d5ec6))
* **pre-commit:** support pre-commit ([a82ca67](https://github.com/xidl/xidl/commit/a82ca6700b7b338133e126869629f12c033f5ae3))
* remove playground ([bae5e45](https://github.com/xidl/xidl/commit/bae5e451a4947b22ca585a6f5d850248bc048774))
* rename *http to *rest ([a9fe5dd](https://github.com/xidl/xidl/commit/a9fe5dd13e426183ee5f8f061d2bb958b18fe91f))
* **rename:** rename typescript-json to typescript-codec and go-json to go-codec ([adb9880](https://github.com/xidl/xidl/commit/adb988056ddfd81472d2272c0c9f4893538ce719))
* reorganization features ([e462694](https://github.com/xidl/xidl/commit/e4626946b3f7821b39381baf6edf21454598be08))
* **rest-hir:** implement enriched HIR mapping and migrate all generators ([1919bf5](https://github.com/xidl/xidl/commit/1919bf50d18ae23bd6ba72ce51b7d04f75beb2cc))
* **rust-axum:** add support for text/plain mime type ([0fdd2d4](https://github.com/xidl/xidl/commit/0fdd2d480da074d7894476bf4a7b3cb93d991b77))
* **rust-axum:** impl bidi_stream ([9b1f1b8](https://github.com/xidl/xidl/commit/9b1f1b883af35016161d33bcd59753a0d319a701))
* **rust-axum:** implement reachability analysis and conditional generation ([d22c08d](https://github.com/xidl/xidl/commit/d22c08dfe10c5455a13fa92726339a79bba938e9))
* **rust-gen:** add --mock flag to generate mockall traits ([bb04fea](https://github.com/xidl/xidl/commit/bb04feaf76858a42371ee52f333416894373d39e))
* **rust-gen:** add support for recursive union types ([cbd0148](https://github.com/xidl/xidl/commit/cbd01481511d380996069fa6a3f1abcb6345797b))
* **rust-jsonrpc:** add conditional compilation for unsupported platforms ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **rust-jsonrpc:** optimize bidirectional method matching ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **skills:** add code-style validation skill ([9422b40](https://github.com/xidl/xidl/commit/9422b407d316dfa381ca4314fb7b36105fdde7f6))
* standardize Default trait and improve annotation support for all Rust types ([491d57e](https://github.com/xidl/xidl/commit/491d57e6d99317a492fb2f786ad12847093f93b5))
* **stream:** using writer on the client_side ([9dc3d1b](https://github.com/xidl/xidl/commit/9dc3d1bd8c53a76194f5ed19c7ff562c1dd18b9e))
* support [@name](https://github.com/name) on enum members ([909595f](https://github.com/xidl/xidl/commit/909595fae6b450a2b12d25ef74977374d0f14c2d))
* support query param ([bd818c4](https://github.com/xidl/xidl/commit/bd818c4b003776515ba46aef0eb400124ffc1170))
* **ts:** generate interface for ts ([0e0ae3a](https://github.com/xidl/xidl/commit/0e0ae3a63924dc06b6698452e4bcd38edb398567))
* **typescript:** add typescript-xidl-json library for metadata-driven serialization ([48f0327](https://github.com/xidl/xidl/commit/48f0327804fea46d11a80d16a9dd7c2388a10dbf))
* **typescript:** migrate to xidl-typescript-codec for metadata-aware serialization ([#178](https://github.com/xidl/xidl/issues/178)) ([3c7b082](https://github.com/xidl/xidl/commit/3c7b082af74cd2ef296e0ab75d814b070711498a))
* **typescript:** rename package to xidl-typescript-json ([9ed12ff](https://github.com/xidl/xidl/commit/9ed12ff89de3e1a379d7d42a417df447a8a6428e))
* unify golang versions in release-please and remove xidlc-examples from tracking ([d900fef](https://github.com/xidl/xidl/commit/d900fef239dd22c3ca78cee6a9121dee4bdb1200))
* using underscore instead of - in annotation ([01e53bf](https://github.com/xidl/xidl/commit/01e53bf5e2310bd26e49aea6ab53c2b88f175104))
* **website:** add Google Analytics integration via environment variable ([f850c8e](https://github.com/xidl/xidl/commit/f850c8e9029bbef6c7b82e291f65dcb6dc765ed2))
* **website:** support highlight idl ([9d3667e](https://github.com/xidl/xidl/commit/9d3667e9709330c429f72417492cff000a517ff3))
* **website:** update docs ([0400c26](https://github.com/xidl/xidl/commit/0400c267ca1ec9891cdc99a89b603a76bd6d97e0))
* **workflow:** add dynamic release context resolution and manual dispatch support ([f9d0d24](https://github.com/xidl/xidl/commit/f9d0d24a02e5f4a0da40c6731673472673caa5b2))
* **xidl-api-keycloak:** build client ([8d95c39](https://github.com/xidl/xidl/commit/8d95c3921113ca079d20cbed9cad11f9915caa5d))
* **xidl-build:** allow set openapi and openrpc output file name ([59eb939](https://github.com/xidl/xidl/commit/59eb939598f1f1693d2658d50a6c22f1c9ba4aba))
* **xidl-build:** expose more option ([aeb035b](https://github.com/xidl/xidl/commit/aeb035b13d8fec565cd3ce6d3adf75f5df410439))
* **xidl-go-codex:** compatible with standard json ([89b6245](https://github.com/xidl/xidl/commit/89b6245387990c7ee5070bec028aa2afbf843682))
* **xidl-go-json:** add more complex flatten rule ([0c0385b](https://github.com/xidl/xidl/commit/0c0385beca9b4ff9555b17ddd982492285f92d92))
* **xidl-parser, xidlc:** add rename/serialize_name/deserialize_name/rename_all annotations ([5827409](https://github.com/xidl/xidl/commit/5827409a098831c0004841140c60a90b9bd7d8d3))
* **xidl-parser:** add recursive type semantic analysis ([905dc4a](https://github.com/xidl/xidl/commit/905dc4a4ec90ae9dd008c8f34f8ee2665aee6219))
* **xidl-parser:** parse hir IntegerSign and IntegerBoolean ([1029226](https://github.com/xidl/xidl/commit/10292266c6593cea3fbc2f562f11de02226f91e0))
* **xidl-parser:** parse IntegerLiteral ([e744d1a](https://github.com/xidl/xidl/commit/e744d1aedaa69cb6b9cc8ec97551132071025ea3))
* **xidl-rust-axum:** make reqwest as a optional dep ([743d5ad](https://github.com/xidl/xidl/commit/743d5ad917952d9b9a9c3ad9d3bdef115db0dc20))
* **xidl-rust-axum:** update error model ([5712624](https://github.com/xidl/xidl/commit/5712624aa0e8046affd7e39aeacf6775b3e9ea2d))
* **xidlc:** add go and go-http targets ([2c4d4c4](https://github.com/xidl/xidl/commit/2c4d4c4a28076127334da203d5d62ccd14553e64))
* **xidlc:** add openrpc support ([081a4a9](https://github.com/xidl/xidl/commit/081a4a9bd8c2430b3e502db792705593d8fe4142))
* **xidlc:** add python http generators and runtime ([1cdff5a](https://github.com/xidl/xidl/commit/1cdff5a0a8f1f9ce4d7dfda9cead47396a06efb4))
* **xidlc:** add skip cdr codec flag ([8174ae2](https://github.com/xidl/xidl/commit/8174ae2889f23aeb8006d7dd96f8213bbdc8e568))
* **xidlc:** add typescript http server generation ([1781c4a](https://github.com/xidl/xidl/commit/1781c4a751694d7190b52b541f0a62ca64b553cd))
* **xidlc:** clean rust axum warning ([4222f65](https://github.com/xidl/xidl/commit/4222f65e68277b7c097f85a326daac72ce078484))
* **xidlc:** clean warning ([0da710a](https://github.com/xidl/xidl/commit/0da710a81317d923c8eb7fd4d006fd283c5b48bd))
* **xidlc:** decouple axum unary request and response transport ([a1ed93b](https://github.com/xidl/xidl/commit/a1ed93b0a56b85d2aff5fc47b06c60338bd141dc))
* **xidlc:** don't expand interface when generate openapi ([a52c87c](https://github.com/xidl/xidl/commit/a52c87ce2a8ef394ab52f30d075d2030e4102617))
* **xidlc:** dont't generate ts and openapi in axum ([b86dd13](https://github.com/xidl/xidl/commit/b86dd131dbc54fdff55eaf1680385d480d1b3de0))
* **xidlc:** generate service code by default ([d03ccf6](https://github.com/xidl/xidl/commit/d03ccf6f00467ccba89be6d5bf6645d7f73424c3))
* **xidlc:** generate typed authentication parameters and conditional constructors for Rust Axum ([3aae8f6](https://github.com/xidl/xidl/commit/3aae8f6f1f5269d4e26203c81fc06687ef6c86e7))
* **xidlc:** impl cors annotation ([7f6ed42](https://github.com/xidl/xidl/commit/7f6ed42a4b6e0577120c23b9d9f85aac0ddc7287))
* **xidlc:** impl default for rust enum ([f9868ac](https://github.com/xidl/xidl/commit/f9868ac41fdd0644d051a02d597bdea0d7fc017f))
* **xidlc:** impl default for rust struct ([24d996f](https://github.com/xidl/xidl/commit/24d996f872d26203069a8a11be56b22d1f5c90f5))
* **xidlc:** impl stream for axum, ts and openapi ([64e8344](https://github.com/xidl/xidl/commit/64e834425ae2fb6df650d333a840b848d1879f12))
* **xidlc:** implement rename/rename_all annotations for golang ([211a128](https://github.com/xidl/xidl/commit/211a1284c5497f016d1798d28040fd2173343dc9))
* **xidlc:** make fmt as a feature ([4ff313a](https://github.com/xidl/xidl/commit/4ff313a0ec01ce29f0a1a5b9d9a1b71c2e107345))
* **xidlc:** remove c and cpp codegen support ([02bdecb](https://github.com/xidl/xidl/commit/02bdecbd9b648cbd20e90cc2c1c0391cb7710616))
* **xidlc:** remove uncessary attribute ([bcdb9b6](https://github.com/xidl/xidl/commit/bcdb9b65c756fc5cd97f9e8cde18c063821678d6))
* **xidlc:** render constructor by service ([a476311](https://github.com/xidl/xidl/commit/a47631193f39fb52c3b5eb556d3e3a3af0f90bda))
* **xidlc:** render doc ([c4d32e7](https://github.com/xidl/xidl/commit/c4d32e7b69803108c192496a768b3315b20f91af))
* **xidlc:** render doc ([eab28b5](https://github.com/xidl/xidl/commit/eab28b5968b17b122a4a4b6277e79a16f59a25f4))
* **xidlc:** replace rust formatter with prettyplease ([87e1b26](https://github.com/xidl/xidl/commit/87e1b269db59f60a1df38e30a3bd7900dee27273))
* **xidlc:** split typescript http generation ([793ea44](https://github.com/xidl/xidl/commit/793ea44d646b45b18a4b244c3845b6c88a4cda15))
* **xidlc:** support [@rust](https://github.com/rust) annotation ([a8f10d6](https://github.com/xidl/xidl/commit/a8f10d64b2ad6770543c7ea96c37a8086f280bfd))
* **xidlc:** support HIR include expansion ([42a715b](https://github.com/xidl/xidl/commit/42a715bfe6a2b7021f8392aca4e352a69efbb6ef))
* **xidlc:** update format ([0695679](https://github.com/xidl/xidl/commit/0695679d823da801e0473c465b586d026701f21f))
* **xidlc:** update jinja formatter ([41bf57a](https://github.com/xidl/xidl/commit/41bf57af999f5d398528d7a0d0aa9252b6ea8ea5))
* **xidlc:** update REST generators and snapshots to support cross-language BDD requirements ([d8e59c3](https://github.com/xidl/xidl/commit/d8e59c314b03cb7273a378dd0045ee7343ebe672))
* **xidl:** remove timestampe in header ([c04c585](https://github.com/xidl/xidl/commit/c04c585b3a7381282729708583cefacd6f000990))
* **xidl:** set lints.workspace=true ([2e63ba4](https://github.com/xidl/xidl/commit/2e63ba408b6a0d84887f3a8f09e887c13e39cadb))
* **xildc-jsonrpc:** rename feature ([b5e3f9a](https://github.com/xidl/xidl/commit/b5e3f9a503153c4d0105bd6cdd7360f61f9236e8))


### Bug Fixes

* address CI failures by applying formatting and updating Go snapshots ([7b8355b](https://github.com/xidl/xidl/commit/7b8355bf43d807e4d2f46c059dfd17aa5a194862))
* **bdd:** avoid duplicate TS server listeners ([9b0ab5b](https://github.com/xidl/xidl/commit/9b0ab5b95afcde26a419e1008322d302838ab0d3))
* **bdd:** avoid ephemeral port reuse ([30c9a6b](https://github.com/xidl/xidl/commit/30c9a6b00cf2e61564625c9e583f789f5f5229bb))
* **bdd:** clean up server process groups ([7669e99](https://github.com/xidl/xidl/commit/7669e9980cdf625c5e54536f03f94c07ca770623))
* **bdd:** extend rust boilerplate startup wait ([ba1a1de](https://github.com/xidl/xidl/commit/ba1a1de9af67c8565faad7769b0d47bfb9d85cf2))
* **bdd:** reserve ports before starting test servers ([991d933](https://github.com/xidl/xidl/commit/991d933d56574ca4cd429a1ea966717bbb499d6a))
* enable subpath tags for Go modules and update internal dependencies using release-please ([4a5591b](https://github.com/xidl/xidl/commit/4a5591b1f969c5ae73bf48821d3cb2e06193c073))
* fix build on docs.rs ([f591ba3](https://github.com/xidl/xidl/commit/f591ba3f856c2d7be49209617c055929bf7145b4))
* fix build on docs.rs ([44e1efb](https://github.com/xidl/xidl/commit/44e1efb499464085510561657adbe6c25d94dbe7))
* fix cargo publish ([60f576e](https://github.com/xidl/xidl/commit/60f576eae160676e2e2b25306de5c2708da4a968))
* fix err code ([4c66e24](https://github.com/xidl/xidl/commit/4c66e2427e833ab1211d800242390f43ab017ac0))
* fix golang tag ([67737bc](https://github.com/xidl/xidl/commit/67737bc8cf945f5480de3ea77ed06abf42bc649f))
* fix release problem ([c35f9ac](https://github.com/xidl/xidl/commit/c35f9ac0c3fff69d297d2f0508663e8736e07e97))
* fix warning ([f29196c](https://github.com/xidl/xidl/commit/f29196c9eecce22b1c4019e64f25a8e16740e994))
* **formula:** update xidlc.rb SHA256 checksum for v0.32.0 ([3b3c71c](https://github.com/xidl/xidl/commit/3b3c71c05a53b1875b03f0f115a5baebf4cda0df))
* **generator:** resolve clippy warnings for redundant field names in Rust Axum from_request implementation and unused variable in Go template ([4b8f1c3](https://github.com/xidl/xidl/commit/4b8f1c30131b97269b45163fa53853ed0df2e32b))
* **go-http:** correct path parameter pattern replacement logic ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **go-rest:** honor client and server generation flags ([f99999f](https://github.com/xidl/xidl/commit/f99999f2e03da78160d277c38f6d3f21276e0234))
* **go:** add delegated go test targets and ci coverage ([38acb67](https://github.com/xidl/xidl/commit/38acb671537301330bb3199eb6dfed22fc180305))
* **go:** preserve acronyms in type and field names ([d68d9a1](https://github.com/xidl/xidl/commit/d68d9a1e1db7dfde95fe26fbe866bcff58303b34))
* **go:** update GinWriteJSONError calls in examples to match new signature ([dca7e75](https://github.com/xidl/xidl/commit/dca7e758a1502969dafdbe50d94e27fb47342d62))
* **http:** support comma-separated cors annotation params ([c631952](https://github.com/xidl/xidl/commit/c63195210e62db505d4720b69f1d49778c217c5f))
* **http:** wrap void out responses as objects ([b5ef38d](https://github.com/xidl/xidl/commit/b5ef38d454d941c18c98453bf5be492459c77922))
* **idl:** normalize oauth scopes for fmt and pre-commit ([76e3fbc](https://github.com/xidl/xidl/commit/76e3fbcc41cd5762d2beaff70399f355536146b8))
* **jsonrpc:** fix jsonrpc ([ffa6aac](https://github.com/xidl/xidl/commit/ffa6aacf5df26ccd1d39f72f1a38d250178f8220))
* **jsonrpc:** generate arc-backed service servers ([0f1a2ca](https://github.com/xidl/xidl/commit/0f1a2ca65469aa944f9fd297d067e55644df0dd3))
* **openrpc:** fix openrpc generate ([359c228](https://github.com/xidl/xidl/commit/359c2289dac54ae9637f176d031c8cca20fdf7a4))
* **parser:** preserve unknown pragmas in hir ([fffff6e](https://github.com/xidl/xidl/commit/fffff6ed4e2711f21f3d81ba9249cd9482ae185a))
* **python:** fix FastAPI adapter path parameter handling and unit tests ([dd9a562](https://github.com/xidl/xidl/commit/dd9a562d549355255cd985f6da50ab65127aed17))
* **python:** install runtime test dependencies ([60446dc](https://github.com/xidl/xidl/commit/60446dc9cf21d91d5572179ebd58dbc3f3cd0208))
* **python:** repair runtime test target ([75ae2ce](https://github.com/xidl/xidl/commit/75ae2ced7f85e6d7504678f05f25bba235212ff9))
* **python:** resolve FastAPI 422 error by overriding endpoint signature ([b075934](https://github.com/xidl/xidl/commit/b075934b39cfec31d0b38c265011bd147351e470))
* remove gen-hir and gen-typed-ast feature ([c8a14d7](https://github.com/xidl/xidl/commit/c8a14d7719beb3a930a8c4ddf09a3e90d9ce2d31))
* **rest:** ensure SingleValue body shapes for text/plain are not wrapped in a JSON object in TS, Rust, Go, and Python generators ([208e697](https://github.com/xidl/xidl/commit/208e697c29e963b4b13233a69200bd909dda840c))
* **rest:** ensure SingleValue response bodies for text/plain are mapped to type aliases instead of structs in Rust Axum generator and fix test assertions ([cf1924b](https://github.com/xidl/xidl/commit/cf1924bb43fa73e41a9e09a403a90391be134275))
* **rest:** skip strict Accept header validation unless explicitly configured ([2b16ee7](https://github.com/xidl/xidl/commit/2b16ee707423f2510d8a517b033a5daeb61ed353))
* **rust-axum:** fix template ([bac9a85](https://github.com/xidl/xidl/commit/bac9a85412b81c28e480edc70138dcc7a02d74b4))
* **rust:** qualify generated BTreeMap paths ([6ff8293](https://github.com/xidl/xidl/commit/6ff8293950da728bf5801e87ac2dce028cc2a861))
* **ts:** resolve BDD test failures and improve REST generator stability ([6c3dd99](https://github.com/xidl/xidl/commit/6c3dd99305aae69d978937ef80ad94f22ecd399c))
* **typeobject:** restore generation and compilation for typeobject idl ([6bc389a](https://github.com/xidl/xidl/commit/6bc389aba4797b97f3f0a9d7576ac82696f0cb73))
* typo ([92ed86b](https://github.com/xidl/xidl/commit/92ed86b742fc9840337d0d75a4e5a9c7b94e9ed0))
* **website:** fix twcss support ([7aff46d](https://github.com/xidl/xidl/commit/7aff46d137adb5f24bf1f3202e419c682c4c45b4))
* **workflow:** enhance release metadata resolution with version parsing ([3b3c71c](https://github.com/xidl/xidl/commit/3b3c71c05a53b1875b03f0f115a5baebf4cda0df))
* **workflow:** ensure publish-release workflow runs on all events ([fd99ec7](https://github.com/xidl/xidl/commit/fd99ec79216bf49b31f6554adff2f7a0dd6a99f7))
* **xidl-parser:** support positional path in HTTP verb annotations ([45f1356](https://github.com/xidl/xidl/commit/45f13560f1615ceb8ca47e73c74ef58de013c16a))
* **xidlc-examples:** address clippy warning in rest_snapshots test ([07b40c5](https://github.com/xidl/xidl/commit/07b40c548d4217266a2d7de5d4df68356475e2ae))
* **xidlc:** add default impls for rust unions ([4b19bea](https://github.com/xidl/xidl/commit/4b19beaf343e7cd203431be8d36f671aaeb5b1ab))
* **xidlc:** add version and git hash to generated headers ([dbfce97](https://github.com/xidl/xidl/commit/dbfce977bf363d6c7c17d4a2b1c10a07234ebdca))
* **xidlc:** address clippy warnings in generator and importer ([bb5bb01](https://github.com/xidl/xidl/commit/bb5bb01e099bbf6c5a64745dab12c1e49c84e9ed))
* **xidlc:** correct jinja formatter block indentation ([944ed1f](https://github.com/xidl/xidl/commit/944ed1f6fe1545b415a294da696a3503b0cf4bcc))
* **xidlc:** fix axum generate ([da8c841](https://github.com/xidl/xidl/commit/da8c841e8350e4705b68a5a08c8916a689e7e69b))
* **xidlc:** fix return value with sequence ([0173ef3](https://github.com/xidl/xidl/commit/0173ef39f6b0dd72108c3bf048cf27db495f2e44))
* **xidlc:** fix wasm build ([782df53](https://github.com/xidl/xidl/commit/782df53b8ba70cc40e08b0dcbf534b29a506e162))
* **xidlc:** generate pointer types and correct bindings for optional Go REST parameters ([a40c8b0](https://github.com/xidl/xidl/commit/a40c8b058f74308b88aaab78fbc6069165d77efc))
* **xidlc:** import referenced model schemas in iface.zod.ts (close [#172](https://github.com/xidl/xidl/issues/172)) ([6b1caa0](https://github.com/xidl/xidl/commit/6b1caa06054032d1ed2e38c76a50d49b1a5d7592))
* **xidlc:** map rust any to serde_json value ([7376020](https://github.com/xidl/xidl/commit/7376020ec717ccc6bd33588d5d27bee76e470592))
* **xidlc:** move rust-axum transport rendering into template context ([d31969c](https://github.com/xidl/xidl/commit/d31969c8adc568d1f4dce41aa3b6ca7aa0564304))
* **xidlc:** resync ipc jsonrpc bindings ([3287c39](https://github.com/xidl/xidl/commit/3287c39e59bc739d06290d1bb3f6920f735e6617))
* **xidlc:** serialize rust unions with real string tags ([a6cb9e9](https://github.com/xidl/xidl/commit/a6cb9e9fd57c19d8508d0ea9a5d417ee450bc182))
* **xidlc:** support standalone JSON Schema and  in openapi import ([6ea0946](https://github.com/xidl/xidl/commit/6ea0946b975371611fdae9873ba88bd1b04271b1))
* **xidlc:** use item-level allow attrs in rust output ([906149e](https://github.com/xidl/xidl/commit/906149eb4c65d8e28dbdad4922e3831f0d7235e2))
* **xidl:** fix multi interface in same file ([02fe850](https://github.com/xidl/xidl/commit/02fe8503e0822bcfa2b298da641e3d6810317753))
* **xidl:** fix package name ([a406176](https://github.com/xidl/xidl/commit/a4061764baee5212eba56b0117e1472998d666af))

## [0.80.1](https://github.com/xidl/xidl/compare/v0.80.0...v0.80.1) (2026-06-18)


### Bug Fixes

* **go-rest:** honor client and server generation flags ([f99999f](https://github.com/xidl/xidl/commit/f99999f2e03da78160d277c38f6d3f21276e0234))

## [0.80.0](https://github.com/xidl/xidl/compare/v0.79.1...v0.80.0) (2026-06-18)


### Features

* **xidl-go-codex:** compatible with standard json ([89b6245](https://github.com/xidl/xidl/commit/89b6245387990c7ee5070bec028aa2afbf843682))

## [0.79.1](https://github.com/xidl/xidl/compare/v0.79.0...v0.79.1) (2026-06-12)


### Bug Fixes

* enable subpath tags for Go modules and update internal dependencies using release-please ([4a5591b](https://github.com/xidl/xidl/commit/4a5591b1f969c5ae73bf48821d3cb2e06193c073))

## [0.79.0](https://github.com/xidl/xidl/compare/v0.78.1...v0.79.0) (2026-06-12)


### Features

* unify golang versions in release-please and remove xidlc-examples from tracking ([d900fef](https://github.com/xidl/xidl/commit/d900fef239dd22c3ca78cee6a9121dee4bdb1200))


### Bug Fixes

* fix err code ([4c66e24](https://github.com/xidl/xidl/commit/4c66e2427e833ab1211d800242390f43ab017ac0))
* fix golang tag ([67737bc](https://github.com/xidl/xidl/commit/67737bc8cf945f5480de3ea77ed06abf42bc649f))

## [0.78.1](https://github.com/xidl/xidl/compare/v0.78.0...v0.78.1) (2026-06-10)


### Bug Fixes

* **xidl:** fix package name ([a406176](https://github.com/xidl/xidl/commit/a4061764baee5212eba56b0117e1472998d666af))

## [0.78.0](https://github.com/xidl/xidl/compare/v0.77.0...v0.78.0) (2026-06-09)


### Features

* **http:** implement raw text serialization for primitive types and update BDD tests ([a72654f](https://github.com/xidl/xidl/commit/a72654f59651c7866e78ee12e7b09fb52497cd5b))
* **http:** standardize error response format to {code, msg} and add BDD bad path tests ([a7cc8d3](https://github.com/xidl/xidl/commit/a7cc8d320968e77a05cdaf8de304264ac423b1ca))


### Bug Fixes

* **bdd:** avoid duplicate TS server listeners ([9b0ab5b](https://github.com/xidl/xidl/commit/9b0ab5b95afcde26a419e1008322d302838ab0d3))
* **bdd:** avoid ephemeral port reuse ([30c9a6b](https://github.com/xidl/xidl/commit/30c9a6b00cf2e61564625c9e583f789f5f5229bb))
* **bdd:** clean up server process groups ([7669e99](https://github.com/xidl/xidl/commit/7669e9980cdf625c5e54536f03f94c07ca770623))
* **bdd:** extend rust boilerplate startup wait ([ba1a1de](https://github.com/xidl/xidl/commit/ba1a1de9af67c8565faad7769b0d47bfb9d85cf2))
* **bdd:** reserve ports before starting test servers ([991d933](https://github.com/xidl/xidl/commit/991d933d56574ca4cd429a1ea966717bbb499d6a))
* **generator:** resolve clippy warnings for redundant field names in Rust Axum from_request implementation and unused variable in Go template ([4b8f1c3](https://github.com/xidl/xidl/commit/4b8f1c30131b97269b45163fa53853ed0df2e32b))
* **go:** update GinWriteJSONError calls in examples to match new signature ([dca7e75](https://github.com/xidl/xidl/commit/dca7e758a1502969dafdbe50d94e27fb47342d62))
* **python:** install runtime test dependencies ([60446dc](https://github.com/xidl/xidl/commit/60446dc9cf21d91d5572179ebd58dbc3f3cd0208))
* **python:** repair runtime test target ([75ae2ce](https://github.com/xidl/xidl/commit/75ae2ced7f85e6d7504678f05f25bba235212ff9))
* **rest:** ensure SingleValue body shapes for text/plain are not wrapped in a JSON object in TS, Rust, Go, and Python generators ([208e697](https://github.com/xidl/xidl/commit/208e697c29e963b4b13233a69200bd909dda840c))
* **rest:** ensure SingleValue response bodies for text/plain are mapped to type aliases instead of structs in Rust Axum generator and fix test assertions ([cf1924b](https://github.com/xidl/xidl/commit/cf1924bb43fa73e41a9e09a403a90391be134275))

## [0.77.0](https://github.com/xidl/xidl/compare/v0.76.1...v0.77.0) (2026-06-04)


### Features

* **xidlc:** update REST generators and snapshots to support cross-language BDD requirements ([d8e59c3](https://github.com/xidl/xidl/commit/d8e59c314b03cb7273a378dd0045ee7343ebe672))

## [0.76.1](https://github.com/xidl/xidl/compare/v0.76.0...v0.76.1) (2026-06-03)


### Bug Fixes

* address CI failures by applying formatting and updating Go snapshots ([7b8355b](https://github.com/xidl/xidl/commit/7b8355bf43d807e4d2f46c059dfd17aa5a194862))
* **ts:** resolve BDD test failures and improve REST generator stability ([6c3dd99](https://github.com/xidl/xidl/commit/6c3dd99305aae69d978937ef80ad94f22ecd399c))

## [0.76.0](https://github.com/xidl/xidl/compare/v0.75.0...v0.76.0) (2026-06-02)


### Features

* **typescript:** migrate to xidl-typescript-codec for metadata-aware serialization ([#178](https://github.com/xidl/xidl/issues/178)) ([3c7b082](https://github.com/xidl/xidl/commit/3c7b082af74cd2ef296e0ab75d814b070711498a))

## [0.75.0](https://github.com/xidl/xidl/compare/v0.74.0...v0.75.0) (2026-06-02)


### Features

* **rename:** rename typescript-json to typescript-codec and go-json to go-codec ([adb9880](https://github.com/xidl/xidl/commit/adb988056ddfd81472d2272c0c9f4893538ce719))
* **typescript:** add typescript-xidl-json library for metadata-driven serialization ([48f0327](https://github.com/xidl/xidl/commit/48f0327804fea46d11a80d16a9dd7c2388a10dbf))
* **typescript:** rename package to xidl-typescript-json ([9ed12ff](https://github.com/xidl/xidl/commit/9ed12ff89de3e1a379d7d42a417df447a8a6428e))

## [0.74.0](https://github.com/xidl/xidl/compare/v0.73.0...v0.74.0) (2026-06-02)


### Features

* **golang:** add xidl-go-json reflection-based library ([c8df2b2](https://github.com/xidl/xidl/commit/c8df2b2d66eb96e0490a45a750ba07b3e6b36f5c))
* **golang:** support catch-all map and any flatten fields in xidl-go-json ([1fa0619](https://github.com/xidl/xidl/commit/1fa0619831475deb8bd6cabbeafe4af7477e3f72))
* **golang:** support flatten tag in xidl-go-json ([d8f428d](https://github.com/xidl/xidl/commit/d8f428d3a5ab45673fd4c68527341a6fbdafa7dc))
* **golang:** use xidl-go-json codec in go-rest and emit xjson tags in xidlc ([772f7ff](https://github.com/xidl/xidl/commit/772f7ffb1916975d5d896fd36ca0c033cb51a2d4))
* **xidl-go-json:** add more complex flatten rule ([0c0385b](https://github.com/xidl/xidl/commit/0c0385beca9b4ff9555b17ddd982492285f92d92))

## [0.73.0](https://github.com/xidl/xidl/compare/v0.72.3...v0.73.0) (2026-06-01)


### Features

* **website:** add Google Analytics integration via environment variable ([f850c8e](https://github.com/xidl/xidl/commit/f850c8e9029bbef6c7b82e291f65dcb6dc765ed2))


### Bug Fixes

* **xidlc:** import referenced model schemas in iface.zod.ts (close [#172](https://github.com/xidl/xidl/issues/172)) ([6b1caa0](https://github.com/xidl/xidl/commit/6b1caa06054032d1ed2e38c76a50d49b1a5d7592))

## [0.72.3](https://github.com/xidl/xidl/compare/v0.72.2...v0.72.3) (2026-05-31)


### Bug Fixes

* **python:** fix FastAPI adapter path parameter handling and unit tests ([dd9a562](https://github.com/xidl/xidl/commit/dd9a562d549355255cd985f6da50ab65127aed17))
* **python:** resolve FastAPI 422 error by overriding endpoint signature ([b075934](https://github.com/xidl/xidl/commit/b075934b39cfec31d0b38c265011bd147351e470))

## [0.72.2](https://github.com/xidl/xidl/compare/v0.72.1...v0.72.2) (2026-05-28)


### Bug Fixes

* **xidl-parser:** support positional path in HTTP verb annotations ([45f1356](https://github.com/xidl/xidl/commit/45f13560f1615ceb8ca47e73c74ef58de013c16a))

## [0.72.1](https://github.com/xidl/xidl/compare/v0.72.0...v0.72.1) (2026-05-28)


### Bug Fixes

* **xidlc:** support standalone JSON Schema and  in openapi import ([6ea0946](https://github.com/xidl/xidl/commit/6ea0946b975371611fdae9873ba88bd1b04271b1))

## [0.72.0](https://github.com/xidl/xidl/compare/v0.71.1...v0.72.0) (2026-05-26)


### Features

* **gen:** change default openapi filename to openapi_{filename}.json ([10c95c3](https://github.com/xidl/xidl/commit/10c95c30eae3e75a3d28e36d5a0f4ddeb7fb2895))

## [0.71.1](https://github.com/xidl/xidl/compare/v0.71.0...v0.71.1) (2026-05-25)


### Bug Fixes

* **xidlc:** add version and git hash to generated headers ([dbfce97](https://github.com/xidl/xidl/commit/dbfce977bf363d6c7c17d4a2b1c10a07234ebdca))

## [0.71.0](https://github.com/xidl/xidl/compare/v0.70.0...v0.71.0) (2026-05-23)


### Features

* add xidl-api-github and openapi importer ([d0475ba](https://github.com/xidl/xidl/commit/d0475bae7ef5a5bfa1eaee26fd749647ef6a18b4))
* **rust-axum:** implement reachability analysis and conditional generation ([d22c08d](https://github.com/xidl/xidl/commit/d22c08dfe10c5455a13fa92726339a79bba938e9))


### Bug Fixes

* **xidlc:** address clippy warnings in generator and importer ([bb5bb01](https://github.com/xidl/xidl/commit/bb5bb01e099bbf6c5a64745dab12c1e49c84e9ed))

## [0.70.0](https://github.com/xidl/xidl/compare/v0.69.2...v0.70.0) (2026-05-23)


### Features

* add xidl-api-discord ([c4cb195](https://github.com/xidl/xidl/commit/c4cb195486619caf54d21f73277934394ff93916))
* add xidl-apis-reddit ([5274a9c](https://github.com/xidl/xidl/commit/5274a9cf1c506c8f249e0d759f4fa7423424750f))
* **keycloak:** update keycloak metadata ([795e85d](https://github.com/xidl/xidl/commit/795e85d5a21857da6b7e519d8826258348e29618))

## [0.69.2](https://github.com/xidl/xidl/compare/v0.69.1...v0.69.2) (2026-05-22)


### Bug Fixes

* fix build on docs.rs ([f591ba3](https://github.com/xidl/xidl/commit/f591ba3f856c2d7be49209617c055929bf7145b4))

## [0.69.1](https://github.com/xidl/xidl/compare/v0.69.0...v0.69.1) (2026-05-22)


### Bug Fixes

* fix build on docs.rs ([44e1efb](https://github.com/xidl/xidl/commit/44e1efb499464085510561657adbe6c25d94dbe7))

## [0.69.0](https://github.com/xidl/xidl/compare/v0.68.0...v0.69.0) (2026-05-22)


### Features

* add keycloak ([47ca2e3](https://github.com/xidl/xidl/commit/47ca2e3f0752c834f021a113f92c6172b4e55555))
* **xidl-api-keycloak:** build client ([8d95c39](https://github.com/xidl/xidl/commit/8d95c3921113ca079d20cbed9cad11f9915caa5d))

## [0.68.0](https://github.com/xidl/xidl/compare/v0.67.0...v0.68.0) (2026-05-22)


### Features

* **xidlc:** generate typed authentication parameters and conditional constructors for Rust Axum ([3aae8f6](https://github.com/xidl/xidl/commit/3aae8f6f1f5269d4e26203c81fc06687ef6c86e7))


### Bug Fixes

* **xidlc:** generate pointer types and correct bindings for optional Go REST parameters ([a40c8b0](https://github.com/xidl/xidl/commit/a40c8b058f74308b88aaab78fbc6069165d77efc))

## [0.67.0](https://github.com/xidl/xidl/compare/v0.66.0...v0.67.0) (2026-05-21)


### Features

* **go:** add support for [@cors](https://github.com/cors) annotation ([253ce87](https://github.com/xidl/xidl/commit/253ce874cd2aed65f2362e4d1c4fb6d0c0cd8d0e))
* **go:** migrate go-rest generator from net/http to gin ([d682fc9](https://github.com/xidl/xidl/commit/d682fc9615d43f42a56e3abb8e5b9368bf487b5b))

## [0.66.0](https://github.com/xidl/xidl/compare/v0.65.0...v0.66.0) (2026-05-21)


### Features

* add ugprade ([ea48027](https://github.com/xidl/xidl/commit/ea48027f36a34d1befe8f82c8666b07bfbfa35f4))
* add upgrade annotation support ([b534408](https://github.com/xidl/xidl/commit/b5344084ccc9e4680c97a240b914b31330ee04cf))

## [0.65.0](https://github.com/xidl/xidl/compare/v0.64.0...v0.65.0) (2026-05-21)


### Features

* **xidlc:** remove uncessary attribute ([bcdb9b6](https://github.com/xidl/xidl/commit/bcdb9b65c756fc5cd97f9e8cde18c063821678d6))
* **xidl:** remove timestampe in header ([c04c585](https://github.com/xidl/xidl/commit/c04c585b3a7381282729708583cefacd6f000990))


### Bug Fixes

* **typeobject:** restore generation and compilation for typeobject idl ([6bc389a](https://github.com/xidl/xidl/commit/6bc389aba4797b97f3f0a9d7576ac82696f0cb73))

## [0.64.0](https://github.com/xidl/xidl/compare/v0.63.0...v0.64.0) (2026-05-20)


### Features

* add default Rust derives for struct and enum ([b25434b](https://github.com/xidl/xidl/commit/b25434bb9006bad056eabe917801506980521b3f))
* standardize Default trait and improve annotation support for all Rust types ([491d57e](https://github.com/xidl/xidl/commit/491d57e6d99317a492fb2f786ad12847093f93b5))

## [0.63.0](https://github.com/xidl/xidl/compare/v0.62.0...v0.63.0) (2026-05-20)


### Features

* **docs:** translate REST and RFC documentation to English ([10b3187](https://github.com/xidl/xidl/commit/10b3187a8727c915d5fd31dfea19f672ed813267))

## [0.62.0](https://github.com/xidl/xidl/compare/v0.61.0...v0.62.0) (2026-05-19)


### Features

* add [@skip](https://github.com/skip) annotation to support skipping fields during serialization ([7cbf6c8](https://github.com/xidl/xidl/commit/7cbf6c80365acf4f9f50c89e6547d973f8c2a6c9))
* **xidlc:** don't expand interface when generate openapi ([a52c87c](https://github.com/xidl/xidl/commit/a52c87ce2a8ef394ab52f30d075d2030e4102617))

## [0.61.0](https://github.com/xidl/xidl/compare/v0.60.1...v0.61.0) (2026-05-19)


### Features

* **xidl-parser, xidlc:** add rename/serialize_name/deserialize_name/rename_all annotations ([5827409](https://github.com/xidl/xidl/commit/5827409a098831c0004841140c60a90b9bd7d8d3))
* **xidlc:** implement rename/rename_all annotations for golang ([211a128](https://github.com/xidl/xidl/commit/211a1284c5497f016d1798d28040fd2173343dc9))

## [0.60.1](https://github.com/xidl/xidl/compare/v0.60.0...v0.60.1) (2026-05-14)


### Bug Fixes

* **http:** support comma-separated cors annotation params ([c631952](https://github.com/xidl/xidl/commit/c63195210e62db505d4720b69f1d49778c217c5f))

## [0.60.0](https://github.com/xidl/xidl/compare/v0.59.0...v0.60.0) (2026-05-14)


### Features

* **xidlc:** impl cors annotation ([7f6ed42](https://github.com/xidl/xidl/commit/7f6ed42a4b6e0577120c23b9d9f85aac0ddc7287))

## [0.59.0](https://github.com/xidl/xidl/compare/v0.58.0...v0.59.0) (2026-05-13)


### Features

* **rest-hir:** implement enriched HIR mapping and migrate all generators ([1919bf5](https://github.com/xidl/xidl/commit/1919bf50d18ae23bd6ba72ce51b7d04f75beb2cc))
* **xidlc:** clean rust axum warning ([4222f65](https://github.com/xidl/xidl/commit/4222f65e68277b7c097f85a326daac72ce078484))
* **xidlc:** clean warning ([0da710a](https://github.com/xidl/xidl/commit/0da710a81317d923c8eb7fd4d006fd283c5b48bd))

## [0.58.0](https://github.com/xidl/xidl/compare/v0.57.0...v0.58.0) (2026-05-09)


### Features

* **rust-gen:** add --mock flag to generate mockall traits ([bb04fea](https://github.com/xidl/xidl/commit/bb04feaf76858a42371ee52f333416894373d39e))

## [0.57.0](https://github.com/xidl/xidl/compare/v0.56.0...v0.57.0) (2026-05-09)


### Features

* **rust-gen:** add support for recursive union types ([cbd0148](https://github.com/xidl/xidl/commit/cbd01481511d380996069fa6a3f1abcb6345797b))


### Bug Fixes

* **xidlc-examples:** address clippy warning in rest_snapshots test ([07b40c5](https://github.com/xidl/xidl/commit/07b40c548d4217266a2d7de5d4df68356475e2ae))

## [0.56.0](https://github.com/xidl/xidl/compare/v0.55.0...v0.56.0) (2026-05-07)


### Features

* **xidl-parser:** add recursive type semantic analysis ([905dc4a](https://github.com/xidl/xidl/commit/905dc4a4ec90ae9dd008c8f34f8ee2665aee6219))

## [0.55.0](https://github.com/xidl/xidl/compare/v0.54.0...v0.55.0) (2026-05-07)


### Features

* **xidl-build:** expose more option ([aeb035b](https://github.com/xidl/xidl/commit/aeb035b13d8fec565cd3ce6d3adf75f5df410439))


### Bug Fixes

* **jsonrpc:** generate arc-backed service servers ([0f1a2ca](https://github.com/xidl/xidl/commit/0f1a2ca65469aa944f9fd297d067e55644df0dd3))

## [0.54.0](https://github.com/xidl/xidl/compare/v0.53.1...v0.54.0) (2026-05-06)


### Features

* **docs:** update docs ([e954096](https://github.com/xidl/xidl/commit/e9540961b653c18fe56de3fd2a359337c8f52e3c))
* rename *http to *rest ([a9fe5dd](https://github.com/xidl/xidl/commit/a9fe5dd13e426183ee5f8f061d2bb958b18fe91f))
* **website:** support highlight idl ([9d3667e](https://github.com/xidl/xidl/commit/9d3667e9709330c429f72417492cff000a517ff3))
* **website:** update docs ([0400c26](https://github.com/xidl/xidl/commit/0400c267ca1ec9891cdc99a89b603a76bd6d97e0))


### Bug Fixes

* **website:** fix twcss support ([7aff46d](https://github.com/xidl/xidl/commit/7aff46d137adb5f24bf1f3202e419c682c4c45b4))
* **xidlc:** serialize rust unions with real string tags ([a6cb9e9](https://github.com/xidl/xidl/commit/a6cb9e9fd57c19d8508d0ea9a5d417ee450bc182))

## [0.53.1](https://github.com/xidl/xidl/compare/v0.53.0...v0.53.1) (2026-05-05)


### Bug Fixes

* **xidlc:** add default impls for rust unions ([4b19bea](https://github.com/xidl/xidl/commit/4b19beaf343e7cd203431be8d36f671aaeb5b1ab))
* **xidlc:** use item-level allow attrs in rust output ([906149e](https://github.com/xidl/xidl/commit/906149eb4c65d8e28dbdad4922e3831f0d7235e2))

## [0.53.0](https://github.com/xidl/xidl/compare/v0.52.0...v0.53.0) (2026-05-04)


### Features

* **xidlc:** add typescript http server generation ([1781c4a](https://github.com/xidl/xidl/commit/1781c4a751694d7190b52b541f0a62ca64b553cd))


### Bug Fixes

* **xidlc:** move rust-axum transport rendering into template context ([d31969c](https://github.com/xidl/xidl/commit/d31969c8adc568d1f4dce41aa3b6ca7aa0564304))

## [0.52.0](https://github.com/xidl/xidl/compare/v0.51.0...v0.52.0) (2026-05-04)


### Features

* **xidlc:** split typescript http generation ([793ea44](https://github.com/xidl/xidl/commit/793ea44d646b45b18a4b244c3845b6c88a4cda15))

## [0.51.0](https://github.com/xidl/xidl/compare/v0.50.2...v0.51.0) (2026-05-03)


### Features

* **jsonrpc:** unify client transport around stream ([4eecedc](https://github.com/xidl/xidl/commit/4eecedc57ae839fc6b94b11c0a9c169ed603864d))


### Bug Fixes

* **xidlc:** resync ipc jsonrpc bindings ([3287c39](https://github.com/xidl/xidl/commit/3287c39e59bc739d06290d1bb3f6920f735e6617))

## [0.50.2](https://github.com/xidl/xidl/compare/v0.50.1...v0.50.2) (2026-04-29)


### Bug Fixes

* **xidl:** fix multi interface in same file ([02fe850](https://github.com/xidl/xidl/commit/02fe8503e0822bcfa2b298da641e3d6810317753))

## [0.50.1](https://github.com/xidl/xidl/compare/v0.50.0...v0.50.1) (2026-04-29)


### Bug Fixes

* **xidlc:** fix return value with sequence ([0173ef3](https://github.com/xidl/xidl/commit/0173ef39f6b0dd72108c3bf048cf27db495f2e44))

## [0.50.0](https://github.com/xidl/xidl/compare/v0.49.0...v0.50.0) (2026-04-27)


### Features

* **rust-axum:** add support for text/plain mime type ([0fdd2d4](https://github.com/xidl/xidl/commit/0fdd2d480da074d7894476bf4a7b3cb93d991b77))

## [0.49.0](https://github.com/xidl/xidl/compare/v0.48.0...v0.49.0) (2026-04-25)


### Features

* reorganization features ([e462694](https://github.com/xidl/xidl/commit/e4626946b3f7821b39381baf6edf21454598be08))

## [0.48.0](https://github.com/xidl/xidl/compare/v0.47.0...v0.48.0) (2026-04-23)


### Features

* **xidlc:** decouple axum unary request and response transport ([a1ed93b](https://github.com/xidl/xidl/commit/a1ed93b0a56b85d2aff5fc47b06c60338bd141dc))

## [0.47.0](https://github.com/xidl/xidl/compare/v0.46.0...v0.47.0) (2026-04-22)


### Features

* **xidlc:** render constructor by service ([a476311](https://github.com/xidl/xidl/commit/a47631193f39fb52c3b5eb556d3e3a3af0f90bda))

## [0.46.0](https://github.com/xidl/xidl/compare/v0.45.0...v0.46.0) (2026-04-21)


### Features

* **xidlc:** update jinja formatter ([41bf57a](https://github.com/xidl/xidl/commit/41bf57af999f5d398528d7a0d0aa9252b6ea8ea5))

## [0.45.0](https://github.com/xidl/xidl/compare/v0.44.0...v0.45.0) (2026-04-16)


### Features

* **bitmask:** support bitbound ([270ca60](https://github.com/xidl/xidl/commit/270ca602fe426a595f83cd7adcb5b8da88c0a2a1))
* **hir:** flatten constexpr ([79934b4](https://github.com/xidl/xidl/commit/79934b4d3d2be3d7d92df3f0c7512c7638211e1b))
* **xidl-parser:** parse hir IntegerSign and IntegerBoolean ([1029226](https://github.com/xidl/xidl/commit/10292266c6593cea3fbc2f562f11de02226f91e0))
* **xidl-parser:** parse IntegerLiteral ([e744d1a](https://github.com/xidl/xidl/commit/e744d1aedaa69cb6b9cc8ec97551132071025ea3))


### Bug Fixes

* **parser:** preserve unknown pragmas in hir ([fffff6e](https://github.com/xidl/xidl/commit/fffff6ed4e2711f21f3d81ba9249cd9482ae185a))

## [0.44.0](https://github.com/xidl/xidl/compare/v0.43.0...v0.44.0) (2026-04-15)


### Features

* **xidlc:** impl default for rust struct ([24d996f](https://github.com/xidl/xidl/commit/24d996f872d26203069a8a11be56b22d1f5c90f5))

## [0.43.0](https://github.com/xidl/xidl/compare/v0.42.0...v0.43.0) (2026-04-15)


### Features

* **xidlc:** impl default for rust enum ([f9868ac](https://github.com/xidl/xidl/commit/f9868ac41fdd0644d051a02d597bdea0d7fc017f))

## [0.42.0](https://github.com/xidl/xidl/compare/v0.41.0...v0.42.0) (2026-04-15)


### Features

* **xidlc:** replace rust formatter with prettyplease ([87e1b26](https://github.com/xidl/xidl/commit/87e1b269db59f60a1df38e30a3bd7900dee27273))

## [0.41.0](https://github.com/xidl/xidl/compare/v0.40.0...v0.41.0) (2026-04-10)


### Features

* **skills:** add code-style validation skill ([9422b40](https://github.com/xidl/xidl/commit/9422b407d316dfa381ca4314fb7b36105fdde7f6))
* **xidlc:** add skip cdr codec flag ([8174ae2](https://github.com/xidl/xidl/commit/8174ae2889f23aeb8006d7dd96f8213bbdc8e568))


### Bug Fixes

* **xidlc:** correct jinja formatter block indentation ([944ed1f](https://github.com/xidl/xidl/commit/944ed1f6fe1545b415a294da696a3503b0cf4bcc))

## [0.40.0](https://github.com/xidl/xidl/compare/v0.39.0...v0.40.0) (2026-04-09)


### Features

* add [@cookie](https://github.com/cookie) support for http generators ([50c005b](https://github.com/xidl/xidl/commit/50c005b073f425a0fb24e5ec6d767445cac0443b))
* add [@http](https://github.com/http)(rename) for field serialization ([d7cb3c8](https://github.com/xidl/xidl/commit/d7cb3c88783f0b115d585a4fd7acbb8120b21697))
* add [@name](https://github.com/name) field rename ([bef6497](https://github.com/xidl/xidl/commit/bef64973f20eae1f42eab971757a2049ba0945ec))
* add annotation ([ea21e45](https://github.com/xidl/xidl/commit/ea21e45dcfaf75ba948e192c7a59e7fc95f0f3e5))
* add bitmask, typedec_dcl and etc ([728bdb6](https://github.com/xidl/xidl/commit/728bdb6dad74b379bc7bf6a4ea1eb0e4bd091960))
* add CD workflow and improve release automation ([755d4be](https://github.com/xidl/xidl/commit/755d4be6068188b2763e6976643a61282d1a7d90))
* add debug ([8437275](https://github.com/xidl/xidl/commit/843727591ca6560a2c7874fe8f688780745cb718))
* add doc comment to annotation ([bb1e191](https://github.com/xidl/xidl/commit/bb1e1918d14cc56657777bdf598a031cce209af6))
* add docusaurus ([72209dc](https://github.com/xidl/xidl/commit/72209dc1f991ebbdb6f6590251ede8bf5e676b17))
* add hir ([0a02a17](https://github.com/xidl/xidl/commit/0a02a17854199f3cffcbc2e75540f3d202c8bf2b))
* add Homebrew formula and improve Windows package support ([4ad1161](https://github.com/xidl/xidl/commit/4ad116103c09680e680b1ecaca848cec4e88f477))
* add http hir layer ([0f3a307](https://github.com/xidl/xidl/commit/0f3a307537f96b9d0b9e56739098805c7c45f7fe))
* add hy2 ([b1c0100](https://github.com/xidl/xidl/commit/b1c0100b80d7155804a3001a4e4d53dcd7d6b04b))
* add idlc ([1f1aab4](https://github.com/xidl/xidl/commit/1f1aab484742a802e92defad8c99f1d97714adb1))
* add interface ([252651a](https://github.com/xidl/xidl/commit/252651ad1052b46994bb9c46c3fb33ec0fc940ab))
* add more support for struct ([c9c21b9](https://github.com/xidl/xidl/commit/c9c21b98ae1f3bdf4f34d27ccbaac472f612141b))
* add more test ([4426f8d](https://github.com/xidl/xidl/commit/4426f8d9246d2108738cf1e205fb1f2ad74a2dec))
* add msgpack support ([bc5876c](https://github.com/xidl/xidl/commit/bc5876c5508844b06b5c6980161fa0e0c9a43051))
* Add OpenAPI 3.2 HTTP stream support ([7d285d6](https://github.com/xidl/xidl/commit/7d285d69b853102c66021cdf96a2a8346fe3f592))
* add playground ([b8663e7](https://github.com/xidl/xidl/commit/b8663e70561c51adfabab5d02162951261872076))
* add typeobject ([fab7a49](https://github.com/xidl/xidl/commit/fab7a49cec8bd943f11cf1c4c891b1e36349ad58))
* add typescript support, fix code gen ([5d65d11](https://github.com/xidl/xidl/commit/5d65d1163a812d12ba1b34ff3e65b07fd27e4603))
* add union and enum ([1a29dd4](https://github.com/xidl/xidl/commit/1a29dd45f63795e482c3e036a4aad1841d6dc9d4))
* add xidl-build ([01dfe3e](https://github.com/xidl/xidl/commit/01dfe3ea7b68e3550e7f20930833f4b9ef72fe52))
* add xidl-rust-axum ([2b40482](https://github.com/xidl/xidl/commit/2b40482a21c942f2a5f9aa605dcf736c483e3509))
* add xidl-xcdr ([4f0394c](https://github.com/xidl/xidl/commit/4f0394cc4ea62a33d80bfaaa1165bc93b36a81d5))
* add xidlc-example ([5ae00fb](https://github.com/xidl/xidl/commit/5ae00fbdc9e6160c83ecd34f9f94b1ba09f71020))
* align unary http and http security mapping ([c0dad07](https://github.com/xidl/xidl/commit/c0dad0728aa735267ecc3715d4ef04871588c015))
* apply [@name](https://github.com/name) to rust struct fields ([ebe42f0](https://github.com/xidl/xidl/commit/ebe42f0efb5ec2a5a69a8576738a0ef66218ba43))
* **axu,:** impl basic-auth ([43ecad3](https://github.com/xidl/xidl/commit/43ecad3abf01d594fc0a51a9f2aec0779522c029))
* **axum:** allow skip_client or skip_server ([6010a45](https://github.com/xidl/xidl/commit/6010a454eb387107b803b2bf0d429c6dd30758a7))
* **axum:** impl auth for client ([9612ce5](https://github.com/xidl/xidl/commit/9612ce5cf4f37627e2c3bc4228d5c8863df2878e))
* **axum:** impl http_bearer ([275c165](https://github.com/xidl/xidl/commit/275c165bb043091c06fb89dbd6c50a0f8aa276d6))
* **axum:** render reqwest error by debug ([6b6908f](https://github.com/xidl/xidl/commit/6b6908f4ed0a344095830bf7ebb9515fd4fe59aa))
* **axum:** support pluggable http body codecs ([f2a8cdd](https://github.com/xidl/xidl/commit/f2a8cdd824cf4ae89fab8b66aa9f9db80e900878))
* **axum:** update attribute generate ([1961984](https://github.com/xidl/xidl/commit/1961984053cb454378057d50fab5358ece270081))
* **axum:** update axum ([e7aef77](https://github.com/xidl/xidl/commit/e7aef774dec984dcb23029fc8b2284ac9625346c))
* **axum:** update axum by http ([00b3743](https://github.com/xidl/xidl/commit/00b374309a8c4693c027f689a39df36604ec8360))
* **bitfield:** support bitfield with fields ([ed00dde](https://github.com/xidl/xidl/commit/ed00dde9a0daeaf506f92b605f38fe68784be2bf))
* bump reqwest to 0.13.2 ([c15dff2](https://github.com/xidl/xidl/commit/c15dff2d4547547129d48ee8f03d577bc8e4ff36))
* bump tree-sitter-idl to 3.15.0 ([f2617c8](https://github.com/xidl/xidl/commit/f2617c85a197e7a7c648aa8f42f2505d05ac11f7))
* bump tree-sitter-idl to 3.16.0 and add annotation support for param_dcl ([60e19ac](https://github.com/xidl/xidl/commit/60e19acb934ffc61fac1f90ad2d5049460c41e37))
* **cargo:** strip file for release ([20c5992](https://github.com/xidl/xidl/commit/20c59920bff114ec31e20f07a7ed15b54810b539))
* **cli:** rename fmt language ([98084f2](https://github.com/xidl/xidl/commit/98084f282fd230ab2446bfd4c1e82d222f932b18))
* complete const_dcl ([f8d56d5](https://github.com/xidl/xidl/commit/f8d56d5eddb918cd0fe5717e09eca2cb1d7dc96c))
* complete http stream and jsonrpc stream ([41373f6](https://github.com/xidl/xidl/commit/41373f6d6fa2e6cb78b4590bfcdc6e45202f1099))
* **const:** impl const ([6493166](https://github.com/xidl/xidl/commit/64931661db2f2f49663ec5d353cc5270de96fa78))
* **const:** support socped name ([65cdae2](https://github.com/xidl/xidl/commit/65cdae2b109807122ae8f2206604710c1b696d44))
* **diagnostic:** refactor code ([cb97ff1](https://github.com/xidl/xidl/commit/cb97ff1b4932b35889edeee13d320edbae98dd9b))
* first commit ([c37354d](https://github.com/xidl/xidl/commit/c37354d2444e16807695ed586b818b5a8b8a0975))
* **fmt:** donot allow format error when test ([6e71122](https://github.com/xidl/xidl/commit/6e7112272b432467cb16e5a20116df843daaebf2))
* **git:** add Cargo.lock ([87b3497](https://github.com/xidl/xidl/commit/87b3497fed31d92f57b0b1b603f75f87333392bd))
* **go-rest:** complete HTTP RFC support ([f9f7896](https://github.com/xidl/xidl/commit/f9f7896f63ae92d4cbfb6f0da96cec8f9c0f2aec))
* **hir:** add const and interface mappings ([3a026c7](https://github.com/xidl/xidl/commit/3a026c731a4405ccf79cbc13c71f06d9121e23dc))
* **hir:** add field_id ([2873091](https://github.com/xidl/xidl/commit/28730911be8a0341fc0aa8813632fff95b7c68c3))
* **hir:** add more implement ([8018bd2](https://github.com/xidl/xidl/commit/8018bd26483fc385974adddd3fe8577e3f0e5684))
* **hir:** add template type specs ([7ae9a09](https://github.com/xidl/xidl/commit/7ae9a09f4de88eee060ba167ce3194b2a855543e))
* **hir:** add union and bitset conversions ([eb3d1e1](https://github.com/xidl/xidl/commit/eb3d1e1235c9843b4f0d246dcf442a3ef70625ec))
* **hir:** support interface ([42371ca](https://github.com/xidl/xidl/commit/42371cacea4f7d872aee42c0fa799ef86067f5fb))
* **hir:** types don't relay on typed_ast ([06e760a](https://github.com/xidl/xidl/commit/06e760a9d121237cf68bba3f1ab3e8eb9ad34741))
* **http:** add [@header](https://github.com/header) support ([d01783a](https://github.com/xidl/xidl/commit/d01783a207c5a90cc2ea442670925e85d3755c5e))
* **http:** add body and flatten annotations ([8753d92](https://github.com/xidl/xidl/commit/8753d929c5ad9829e796700b39d781b1689f88ce))
* **http:** document the in, out and inout ([70c2d1b](https://github.com/xidl/xidl/commit/70c2d1bdde2b16622f0ad573bcdf5a0f25555e06))
* **idlc:** add bitmask, bitset and union ([4b52183](https://github.com/xidl/xidl/commit/4b52183d93992752d4940e634d1172547475d7fc))
* **idlc:** add rust module support ([07680b0](https://github.com/xidl/xidl/commit/07680b011a5ed464e3e712b7bbd1faa33dada2a8))
* **idlc:** add support for more serialize format ([0d92a56](https://github.com/xidl/xidl/commit/0d92a564b29fae5ce1033829889aa623afdaa590))
* **idlc:** better serialize ([f15a2c8](https://github.com/xidl/xidl/commit/f15a2c87715219ff09bb848611d85c8461698739))
* impl some idlc ([cf5407b](https://github.com/xidl/xidl/commit/cf5407b34f23337d119ce81090481315f39073e1))
* **jsonpc:** add tcp and inproc support ([e6c7b93](https://github.com/xidl/xidl/commit/e6c7b93a32f5aaed976d38622ae402c202fd5352))
* **jsonrpc:** add example ([fad7c8b](https://github.com/xidl/xidl/commit/fad7c8b48d85c93e274047853bc7b54e56c643df))
* **jsonrpc:** add ipc transport for plugins ([5d9cfdb](https://github.com/xidl/xidl/commit/5d9cfdbf026474594f71303ab2a8c22a5ab9b9d0))
* **jsonrpc:** add quic support ([c07e660](https://github.com/xidl/xidl/commit/c07e6605b2cc533eee5005345650048217fc08cb))
* **jsonrpc:** add ws, wss, tls support ([fc98fdc](https://github.com/xidl/xidl/commit/fc98fdc7781bdff5f3f9dfeabe61764457109096))
* **jsonrpc:** expose bound server endpoint ([a933984](https://github.com/xidl/xidl/commit/a9339842cf72655d2670dc6a5eafcd4ad95840e5))
* **jsonrpc:** impl stream for jsonrpc ([df85fe5](https://github.com/xidl/xidl/commit/df85fe5ee9b241047eb265352eb4162cbe5bb6fa))
* **jsonrpc:** make jsonrpc tokio as optional ([17e1227](https://github.com/xidl/xidl/commit/17e1227c0c757d165e16ad4bf2eacfbf7309ed79))
* **jsonrpc:** refactor code ([81f7ec8](https://github.com/xidl/xidl/commit/81f7ec80f314e004351ba6ec026edf65151b19c0))
* **jsonrpc:** update jsonrpc ([6528364](https://github.com/xidl/xidl/commit/65283641166bf93a19d98a26072601b02d4e83b9))
* **jsonrpc:** update jsonrpc ([812ed3e](https://github.com/xidl/xidl/commit/812ed3e472d5d6bbca6327e0ce4d0777e5dff291))
* **jsonrpc:** update rust_jsonrpc ([66f9448](https://github.com/xidl/xidl/commit/66f9448bb73261edef8bc487a78f9cf9fb6b5d0c))
* **jsonrpc:** using async_trait ([f961489](https://github.com/xidl/xidl/commit/f9614893ad503f26853d638932c05b0fbd8c15de))
* **jsonrpc:** using enum instead i64 ([22fcd82](https://github.com/xidl/xidl/commit/22fcd827684afbe24e4717f918329a94c3118137))
* make build faster ([908a6d7](https://github.com/xidl/xidl/commit/908a6d74bacdd7614a8fbea1551fad6a54163e84))
* make it works ([faff2bc](https://github.com/xidl/xidl/commit/faff2bcd1c1035f85a7d999159d86929906ad65e))
* **openapi:** auto select openapi version ([6aad440](https://github.com/xidl/xidl/commit/6aad4400f232db1e2017983e7ccdb07015673e30))
* **openapi:** support progma xidlc service ([649f542](https://github.com/xidl/xidl/commit/649f542540940b2ccf013e72c2613243ce54f87c))
* **openapi:** unify return code ([609dc5a](https://github.com/xidl/xidl/commit/609dc5a7d6ca3e85d8022ab6cfef676954b27b16))
* parse some base-types ([9c60a96](https://github.com/xidl/xidl/commit/9c60a96f79aa16c934068800674a23362e281ca2))
* **parser:** add serialize, deserialize for typed_ast ([2e412b0](https://github.com/xidl/xidl/commit/2e412b0b3aa66b739b1d2c4ed6620ac347294111))
* **parser:** handle error node ([dff08b8](https://github.com/xidl/xidl/commit/dff08b8a5611b706587441488e458c0bfba1519d))
* **parser:** support extend_annonation ([af73f64](https://github.com/xidl/xidl/commit/af73f641fe3bd59783a46d7baa6677ce9e10fd51))
* pass the first test ([fa634bc](https://github.com/xidl/xidl/commit/fa634bcf5cc0624e9bd65b4fd6df1c4829f3645b))
* **playground:** add format ([49c35e8](https://github.com/xidl/xidl/commit/49c35e8f46c9431058056e66c2ea0d0de48a282d))
* **playground:** add openapi and openrpc ([e68971d](https://github.com/xidl/xidl/commit/e68971d23779c0beafa29ce2a7356050a14d5ec6))
* **playground:** add share button ([541770b](https://github.com/xidl/xidl/commit/541770badfc2cef7078075f13d6cd6cf9e5e2de9))
* **playground:** support share code ([1202df5](https://github.com/xidl/xidl/commit/1202df548df044f5d20fb0ab237b15473764872c))
* **playground:** update playground ([3b2e729](https://github.com/xidl/xidl/commit/3b2e729eaff62080762d5534fb92bad930fee53a))
* **pre-commit:** support pre-commit ([a82ca67](https://github.com/xidl/xidl/commit/a82ca6700b7b338133e126869629f12c033f5ae3))
* remove playground ([bae5e45](https://github.com/xidl/xidl/commit/bae5e451a4947b22ca585a6f5d850248bc048774))
* **rust-axum:** add more error method ([63c0c15](https://github.com/xidl/xidl/commit/63c0c15afd2cc774bdbc47ab2003bb4b8f7619c6))
* **rust-axum:** add serve_with_listener ([a3bdcbc](https://github.com/xidl/xidl/commit/a3bdcbc4765394b5c962df46a8b4564972361d43))
* **rust-axum:** impl bidi_stream ([9b1f1b8](https://github.com/xidl/xidl/commit/9b1f1b883af35016161d33bcd59753a0d319a701))
* **rust-axum:** make error as const ([fc5146b](https://github.com/xidl/xidl/commit/fc5146b9556a3fce5271b08e2103db3bbf5e87be))
* **rust-axum:** support axum ([09c3522](https://github.com/xidl/xidl/commit/09c352289a0517e67c33c44d41f2e4688a8df801))
* **rust-jsonrpc:** add conditional compilation for unsupported platforms ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **rust-jsonrpc:** optimize bidirectional method matching ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **rust:** bumpd rust to 1.92 ([66d8010](https://github.com/xidl/xidl/commit/66d8010ade0169509570907ee1777d989f90fa80))
* **rust:** remove typeobject ([e43969c](https://github.com/xidl/xidl/commit/e43969c7f72eff046c7d1853bf2e65c6c46ff857))
* **rust:** support [@derive](https://github.com/derive) ([5992faf](https://github.com/xidl/xidl/commit/5992faf4141363474c31678b245e2febb5d3928a))
* **scoped_name:** handle is_root ([58a7bcb](https://github.com/xidl/xidl/commit/58a7bcbce6459fdb3b2516728539f03e5173a98e))
* **stream:** using writer on the client_side ([9dc3d1b](https://github.com/xidl/xidl/commit/9dc3d1bd8c53a76194f5ed19c7ff562c1dd18b9e))
* support [@name](https://github.com/name) on enum members ([909595f](https://github.com/xidl/xidl/commit/909595fae6b450a2b12d25ef74977374d0f14c2d))
* support #progma xidlc package and #progma xidlc version ([fa3d418](https://github.com/xidl/xidl/commit/fa3d418921f03f647bd97ead0d75043a948a6c85))
* support float ([49e42c5](https://github.com/xidl/xidl/commit/49e42c57ed2509a66f6dda140cdb5ab01b8f3ce0))
* support more type ([5c42ffc](https://github.com/xidl/xidl/commit/5c42ffc8e71e70d632d890163dcf2a5f3a6a4225))
* support more type ([686e0bc](https://github.com/xidl/xidl/commit/686e0bcbaefece57a56c14511f5836b35b7199f4))
* support more type ([e16119c](https://github.com/xidl/xidl/commit/e16119c2022a66d21405ec67bae2e2238c98b538))
* support query param ([bd818c4](https://github.com/xidl/xidl/commit/bd818c4b003776515ba46aef0eb400124ffc1170))
* **ts:** generate interface for ts ([0e0ae3a](https://github.com/xidl/xidl/commit/0e0ae3a63924dc06b6698452e4bcd38edb398567))
* **typed_ast:** expand corpus coverage ([4236784](https://github.com/xidl/xidl/commit/42367849edbe1e5fbd26d915050a8510fd2c3eaf))
* **union:** add union member support ([eb013cb](https://github.com/xidl/xidl/commit/eb013cb703ce75fc0df4e7f10da3e0b6b7c62352))
* using underscore instead of - in annotation ([01e53bf](https://github.com/xidl/xidl/commit/01e53bf5e2310bd26e49aea6ab53c2b88f175104))
* **workflow:** add dynamic release context resolution and manual dispatch support ([f9d0d24](https://github.com/xidl/xidl/commit/f9d0d24a02e5f4a0da40c6731673472673caa5b2))
* x ([1db79eb](https://github.com/xidl/xidl/commit/1db79ebb11387b2a8eb69a1948d3310e49b506ae))
* **xcdr:** add delimited cdr ([521219f](https://github.com/xidl/xidl/commit/521219f385d7681154ca830d5867dd613811cc54))
* **xcdr:** add plain_cdr2 ([491c4cc](https://github.com/xidl/xidl/commit/491c4cc7e4a96241384f07fbe38511de3b7b3fbc))
* **xcdr:** add plain-cdr ([ef5a65b](https://github.com/xidl/xidl/commit/ef5a65bebd142442b7523429991a277a1dedf729))
* **xcdr:** add plcdr ([7084e64](https://github.com/xidl/xidl/commit/7084e641554e7f62f24619f796cd4e4acae11737))
* **xcdr:** add plcdr2 ([4bd18f5](https://github.com/xidl/xidl/commit/4bd18f5f94133f9306ed2c197307aa8b1c76df35))
* **xcdr:** add xcdr-plcdr ([a5f03ed](https://github.com/xidl/xidl/commit/a5f03eda2aa0e27fb23d07fc9f3c3c23e3d539a1))
* **xcdr:** better ffi ([5e5e301](https://github.com/xidl/xidl/commit/5e5e301ea27dde57dd68d568a50fa104671ae26b))
* **xcdr:** calc EMHEADER ([766df12](https://github.com/xidl/xidl/commit/766df120ca653dfcf206168a5c2e7bde389d2375))
* **xcdr:** reimpl xcdr ([48c063a](https://github.com/xidl/xidl/commit/48c063a58a06f8002f1299094496513c28f42548))
* **xidl-build:** add `with_` prefix for all method ([27979da](https://github.com/xidl/xidl/commit/27979dafc79d943d74cb5afdad5377827987ae7d))
* **xidl-build:** allow set openapi and openrpc output file name ([59eb939](https://github.com/xidl/xidl/commit/59eb939598f1f1693d2658d50a6c22f1c9ba4aba))
* **xidl-rust-axum:** make reqwest as a optional dep ([743d5ad](https://github.com/xidl/xidl/commit/743d5ad917952d9b9a9c3ad9d3bdef115db0dc20))
* **xidl-rust-axum:** update error model ([5712624](https://github.com/xidl/xidl/commit/5712624aa0e8046affd7e39aeacf6775b3e9ea2d))
* **xidl:** add more methods ([ddd54f2](https://github.com/xidl/xidl/commit/ddd54f249238c6c9251cd5b702127b9494898b57))
* **xidl:** add parser attribute ([73ef095](https://github.com/xidl/xidl/commit/73ef0954513d3faf29ad32dda791e2be24b9ab00))
* **xidlc:** add artifact ([139031e](https://github.com/xidl/xidl/commit/139031ede4b2ad1058159e765b5190031fe35b68))
* **xidlc:** add code highlight ([417f3ef](https://github.com/xidl/xidl/commit/417f3efff0b09d040300c6c5c1d1fb6238ea5e07))
* **xidlc:** add cpp formatter ([f495752](https://github.com/xidl/xidl/commit/f495752c2a430ed4eced0fff5c0b4d3199fb8b2c))
* **xidlc:** add cpp serialize ([274cb44](https://github.com/xidl/xidl/commit/274cb44406e1e6b383db3d3747d2424e5c2984c9))
* **xidlc:** add deser code gen ([fe2920a](https://github.com/xidl/xidl/commit/fe2920abaefc9f0f2ee19fe4616c82d21d620a84))
* **xidlc:** add diagnosic module ([c78463a](https://github.com/xidl/xidl/commit/c78463a34d344ed0ba39d32fa53c0d9c57f20369))
* **xidlc:** add dry-run ([529292f](https://github.com/xidl/xidl/commit/529292f79f09121927f7883596aeed450a00e664))
* **xidlc:** add file header ([19cf730](https://github.com/xidl/xidl/commit/19cf730a9852bd2b9e322a8f8e145c5b70f4b6b0))
* **xidlc:** add format ([28d55e4](https://github.com/xidl/xidl/commit/28d55e4744cb25cc5cba22d230c614b001ee2ca4))
* **xidlc:** add get_engine_version method ([567986d](https://github.com/xidl/xidl/commit/567986d7ca0e20a4259fec1ca6405b86fea6ed2c))
* **xidlc:** add go and go-rest targets ([2c4d4c4](https://github.com/xidl/xidl/commit/2c4d4c4a28076127334da203d5d62ccd14553e64))
* **xidlc:** add inplace for format ([9b08504](https://github.com/xidl/xidl/commit/9b0850496a03dcf7e52491ca647676f9357b454c))
* **xidlc:** add interface support ([7899205](https://github.com/xidl/xidl/commit/789920562b0e1e5f2a7e6eecb33cfd4073f7f089))
* **xidlc:** add is_optional to member ([bbc7ed2](https://github.com/xidl/xidl/commit/bbc7ed2cd7ed71a459bdcf0fd6d10edd08a55d91))
* **xidlc:** add jinja formatter ([8808e6e](https://github.com/xidl/xidl/commit/8808e6e29b64a1e54bde3bd0f4aa01bc0ee31b17))
* **xidlc:** add jsonrpc_full ([f2fe6b7](https://github.com/xidl/xidl/commit/f2fe6b7245aa21b9467050841f964917e8601983))
* **xidlc:** add log ([a52b475](https://github.com/xidl/xidl/commit/a52b475a76731db697cd774dde4aa61ae1629100))
* **xidlc:** add more diagnostic ([f49b13e](https://github.com/xidl/xidl/commit/f49b13ea0de1d12f0771890ad9a5343657600abe))
* **xidlc:** add openrpc support ([081a4a9](https://github.com/xidl/xidl/commit/081a4a9bd8c2430b3e502db792705593d8fe4142))
* **xidlc:** add python http generators and runtime ([1cdff5a](https://github.com/xidl/xidl/commit/1cdff5a0a8f1f9ce4d7dfda9cead47396a06efb4))
* **xidlc:** add rust format filter ([25bff71](https://github.com/xidl/xidl/commit/25bff718b0450790ee21982bd1bf9353435a9372))
* **xidlc:** add rust_jsonrpc ([b445719](https://github.com/xidl/xidl/commit/b4457195d45ce5ec71a462b60aadeca816185d75))
* **xidlc:** add spec template ([a85b656](https://github.com/xidl/xidl/commit/a85b6566a8d7341ae71b49b617a9cbac609c6965))
* **xidlc:** add support for [@optional](https://github.com/optional) ([85a75dd](https://github.com/xidl/xidl/commit/85a75dd493d45f84c027080279d63eb494943d12))
* **xidlc:** add support for rust ([0bafa1b](https://github.com/xidl/xidl/commit/0bafa1bd568b6322acb08071faa9230a8f9a0131))
* **xidlc:** add typed_ast_gen ([a630d38](https://github.com/xidl/xidl/commit/a630d389aae0b040ac31ac936422e0a22bfe78a1))
* **xidlc:** add typescript formatter ([838a3ce](https://github.com/xidl/xidl/commit/838a3ce2dade19d6507d932aef42f933e47bc47f))
* **xidlc:** align with rfc ([368f742](https://github.com/xidl/xidl/commit/368f742b8b70531f2f2b7b5ea9c771b4bee048a8))
* **xidlc:** allow generate hir ([7af7d76](https://github.com/xidl/xidl/commit/7af7d76717cc258f96d2d82ff929d2adbc270019))
* **xidlc:** allow skip serialize and deserialize in rust ([b31a956](https://github.com/xidl/xidl/commit/b31a9561778d98874993ee31f59b21652d0433ff))
* **xidlc:** better attribute ([b19d093](https://github.com/xidl/xidl/commit/b19d0933e7ebe3a6911f935c8d0352f335bbbfae))
* **xidlc:** better cli ([ab3e81d](https://github.com/xidl/xidl/commit/ab3e81db5f36e03663dc364d7674b7dcec4eb7cc))
* **xidlc:** better diagnosic ([5953bfd](https://github.com/xidl/xidl/commit/5953bfd021e30c8c2bdabcc5ca1a813dcfa65fc1))
* **xidlc:** better support for rust ([10fd91b](https://github.com/xidl/xidl/commit/10fd91baef24773ef808b0f5105fbacf71ba3fde))
* **xidlc:** diagnostic support filename ([b94d430](https://github.com/xidl/xidl/commit/b94d430f863cf37a46992e5c2e4cc8938b3b0846))
* **xidlc:** dont't generate ts and openapi in axum ([b86dd13](https://github.com/xidl/xidl/commit/b86dd131dbc54fdff55eaf1680385d480d1b3de0))
* **xidlc:** eat the json_rpc food ([9e91e24](https://github.com/xidl/xidl/commit/9e91e240ab2893e1b7b3798fa737d551aa64d7b4))
* **xidlc:** generate service code by default ([d03ccf6](https://github.com/xidl/xidl/commit/d03ccf6f00467ccba89be6d5bf6645d7f73424c3))
* **xidlc:** impl stream for axum, ts and openapi ([64e8344](https://github.com/xidl/xidl/commit/64e834425ae2fb6df650d333a840b848d1879f12))
* **xidlc:** integrated highlighting into diagnostics ([46abbea](https://github.com/xidl/xidl/commit/46abbea1591210f4485d2e44fc2bda5d9bab53f6))
* **xidlc:** make fmt as a feature ([4ff313a](https://github.com/xidl/xidl/commit/4ff313a0ec01ce29f0a1a5b9d9a1b71c2e107345))
* **xidlc:** make openapi as a single language ([6a6f2a1](https://github.com/xidl/xidl/commit/6a6f2a19ec8a7af5d67be9c323a811bbcb47c0f6))
* **xidlc:** make rust union safe ([282f494](https://github.com/xidl/xidl/commit/282f494d429f8d660c0bb8a52798c11240afc769))
* **xidlc:** panic when formate in test ([577c900](https://github.com/xidl/xidl/commit/577c900ad779471b6c57fd9e7a94a73964f5e7d7))
* **xidlc:** print help if non file is provided ([b6c6e8f](https://github.com/xidl/xidl/commit/b6c6e8f732b55bab585110311dd9239a6ecdd9e7))
* **xidlc:** read FINAL and APPENDABLE and etc ([cbb79ee](https://github.com/xidl/xidl/commit/cbb79ee78e838f66fef310263b1e271f0fe998dd))
* **xidlc:** refactor cli fmt ([f41ebe7](https://github.com/xidl/xidl/commit/f41ebe76b9df7ef38aa12e13784dc7498669ddda))
* **xidlc:** remove axum nested object ([7c8262b](https://github.com/xidl/xidl/commit/7c8262bf6ad3e8b8b442cac6022476b5b5112335))
* **xidlc:** remove c and cpp codegen support ([02bdecb](https://github.com/xidl/xidl/commit/02bdecbd9b648cbd20e90cc2c1c0391cb7710616))
* **xidlc:** remove rust module attribute ([11cc7bc](https://github.com/xidl/xidl/commit/11cc7bce57ced8be94be2b4df1c5ad969ac51a31))
* **xidlc:** render doc ([c4d32e7](https://github.com/xidl/xidl/commit/c4d32e7b69803108c192496a768b3315b20f91af))
* **xidlc:** render doc ([eab28b5](https://github.com/xidl/xidl/commit/eab28b5968b17b122a4a4b6277e79a16f59a25f4))
* **xidlc:** rust enum support serialize ([eb34505](https://github.com/xidl/xidl/commit/eb34505cb0aaabdcc5700864c4226c1978b8c6d7))
* **xidlc:** short cmdline ([8961f47](https://github.com/xidl/xidl/commit/8961f470abb2899225dbe285a405f2fffa8c7bfe))
* **xidlc:** support [@rust](https://github.com/rust) annotation ([a8f10d6](https://github.com/xidl/xidl/commit/a8f10d64b2ad6770543c7ea96c37a8086f280bfd))
* **xidlc:** support cpp ([46a2436](https://github.com/xidl/xidl/commit/46a2436340e4fffb7dfd892e06fbf4aab3350243))
* **xidlc:** support HIR include expansion ([42a715b](https://github.com/xidl/xidl/commit/42a715bfe6a2b7021f8392aca4e352a69efbb6ef))
* **xidlc:** switch to async ([1104255](https://github.com/xidl/xidl/commit/1104255463a4f1bcb77b91579f8014714b8b279d))
* **xidlc:** update ([f7cbdbc](https://github.com/xidl/xidl/commit/f7cbdbce906196f22a5537fba6d59a1e66ad4801))
* **xidlc:** update axum and ts by http rfc ([8f1fcd7](https://github.com/xidl/xidl/commit/8f1fcd744f773ff6128497648aa57a3ebf235fd5))
* **xidlc:** update format ([0695679](https://github.com/xidl/xidl/commit/0695679d823da801e0473c465b586d026701f21f))
* **xidlc:** update rust code ([2cd4429](https://github.com/xidl/xidl/commit/2cd4429dfdc28bbec9a4df886cc2364097da3289))
* **xidlc:** update template ([85ef6a1](https://github.com/xidl/xidl/commit/85ef6a1b9997aa6ecf2ce8f2d87a5cc39f1fe48e))
* **xidlc:** using full typepath in rust ([9b647a9](https://github.com/xidl/xidl/commit/9b647a9558266480f65c826c66cb2dbe658335ff))
* **xidlc:** using mempipe for builtin language ([e37956e](https://github.com/xidl/xidl/commit/e37956ea6b75890ef99a2ad9108c3034b9e7dca0))
* **xidlc:** using rpc ([5bd242e](https://github.com/xidl/xidl/commit/5bd242ed3dcda04264f9a34fa61d2b575459ab1e))
* **xidlc:** using the enable but not disable ([8be7465](https://github.com/xidl/xidl/commit/8be7465a22d8e713809f82a4ce4c92af626ec8a5))
* **xidl:** generate file.rs instead mod.rs ([4e6eee7](https://github.com/xidl/xidl/commit/4e6eee72677bbd106d6485b524d7a3b5dc530970))
* **xidl:** set lints.workspace=true ([2e63ba4](https://github.com/xidl/xidl/commit/2e63ba408b6a0d84887f3a8f09e887c13e39cadb))
* **xidl:** split into two files ([1f7cc2f](https://github.com/xidl/xidl/commit/1f7cc2f8603f8cc86fb645816d03ab911edc33f7))
* **xidl:** support catch all path ([96a6540](https://github.com/xidl/xidl/commit/96a654000f9a39b111867dc325664f073e0ad074))
* **xidl:** support template type ([3e27c1a](https://github.com/xidl/xidl/commit/3e27c1a5d67c40135f80377866bd6115a514039e))
* **xidl:** update ts by http ([71a388b](https://github.com/xidl/xidl/commit/71a388b74803f83c7f642f7c14ec0fab14e6bba5))
* **xildc-jsonrpc:** rename feature ([b5e3f9a](https://github.com/xidl/xidl/commit/b5e3f9a503153c4d0105bd6cdd7360f61f9236e8))


### Bug Fixes

* **examples:** fix warnings ([1cc2b66](https://github.com/xidl/xidl/commit/1cc2b66db6b36e9d97f0b4c64f2bf8e82b671f9e))
* fix build problem ([1aede36](https://github.com/xidl/xidl/commit/1aede360162dfefcd79078f3df5808fb849b9770))
* fix cargo publish ([60f576e](https://github.com/xidl/xidl/commit/60f576eae160676e2e2b25306de5c2708da4a968))
* fix release problem ([c35f9ac](https://github.com/xidl/xidl/commit/c35f9ac0c3fff69d297d2f0508663e8736e07e97))
* fix some error ([9b25de5](https://github.com/xidl/xidl/commit/9b25de5dffc99224e998bfcc2d941386f28d2a1c))
* fix warning ([f29196c](https://github.com/xidl/xidl/commit/f29196c9eecce22b1c4019e64f25a8e16740e994))
* **fmt:** fix formate problem ([cdaaa6a](https://github.com/xidl/xidl/commit/cdaaa6a42651410f0a02839380cdaeae597ac354))
* **formula:** update xidlc.rb SHA256 checksum for v0.32.0 ([3b3c71c](https://github.com/xidl/xidl/commit/3b3c71c05a53b1875b03f0f115a5baebf4cda0df))
* **go-rest:** correct path parameter pattern replacement logic ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **go:** add delegated go test targets and ci coverage ([38acb67](https://github.com/xidl/xidl/commit/38acb671537301330bb3199eb6dfed22fc180305))
* **http:** wrap void out responses as objects ([b5ef38d](https://github.com/xidl/xidl/commit/b5ef38d454d941c18c98453bf5be492459c77922))
* **idlc:** fix rust gen ([c908ce8](https://github.com/xidl/xidl/commit/c908ce8ab0de5c448bb25c7e02b225f70d78ff7f))
* **idlc:** rename idlc to xidlc ([1139abe](https://github.com/xidl/xidl/commit/1139abe035497f624b76d9bf2b14d26f9e1cf088))
* **idl:** normalize oauth scopes for fmt and pre-commit ([76e3fbc](https://github.com/xidl/xidl/commit/76e3fbcc41cd5762d2beaff70399f355536146b8))
* **jsonrpc:** fix jsonrpc ([ffa6aac](https://github.com/xidl/xidl/commit/ffa6aacf5df26ccd1d39f72f1a38d250178f8220))
* **openrpc:** fix openrpc generate ([359c228](https://github.com/xidl/xidl/commit/359c2289dac54ae9637f176d031c8cca20fdf7a4))
* remove gen-hir and gen-typed-ast feature ([c8a14d7](https://github.com/xidl/xidl/commit/c8a14d7719beb3a930a8c4ddf09a3e90d9ce2d31))
* **rust-axum:** fix generate ([6ddc701](https://github.com/xidl/xidl/commit/6ddc70162251ece354df22a02093c34312c2b796))
* **rust-axum:** fix template ([bac9a85](https://github.com/xidl/xidl/commit/bac9a85412b81c28e480edc70138dcc7a02d74b4))
* **rust-wasm:** remove openssl dep ([76afac8](https://github.com/xidl/xidl/commit/76afac84647e861f8abe9bccece5645054a285a3))
* **rust:** qualify generated BTreeMap paths ([6ff8293](https://github.com/xidl/xidl/commit/6ff8293950da728bf5801e87ac2dce028cc2a861))
* **scoped_name:** rename identifier to vec ([6eedc2e](https://github.com/xidl/xidl/commit/6eedc2ebfcc1e66ee146e2fee57d97ddeebec4fb))
* **typed_ast:** parse attr raises declarator ([2398cd4](https://github.com/xidl/xidl/commit/2398cd49ee154079ee799efd64dcfd10db21cf16))
* typo ([92ed86b](https://github.com/xidl/xidl/commit/92ed86b742fc9840337d0d75a4e5a9c7b94e9ed0))
* **wasm:** fix xidlc on wasm ([a5805b1](https://github.com/xidl/xidl/commit/a5805b14276b970f9eca127185c003d3343a9d94))
* **workflow:** enhance release metadata resolution with version parsing ([3b3c71c](https://github.com/xidl/xidl/commit/3b3c71c05a53b1875b03f0f115a5baebf4cda0df))
* **workflow:** ensure publish-release workflow runs on all events ([fd99ec7](https://github.com/xidl/xidl/commit/fd99ec79216bf49b31f6554adff2f7a0dd6a99f7))
* **xidl-build:** fix method name ([87509cf](https://github.com/xidl/xidl/commit/87509cf16b4a952738fa47f20bc0e62535eae3ac))
* **xidlc:** fix axum generate ([da8c841](https://github.com/xidl/xidl/commit/da8c841e8350e4705b68a5a08c8916a689e7e69b))
* **xidlc:** fix block for wasm ([2e0300d](https://github.com/xidl/xidl/commit/2e0300d807fb052dd641afdb44f71a89fa86f255))
* **xidlc:** fix build on windows ([26f39a6](https://github.com/xidl/xidl/commit/26f39a6e901bbd054ae919f2866065c2b11cc970))
* **xidlc:** fix c/cpp generate ([878f460](https://github.com/xidl/xidl/commit/878f460e5a86b3c96207a6d4554f39d803a129ed))
* **xidlc:** fix rust crate name ([90bc773](https://github.com/xidl/xidl/commit/90bc7732b2fe277a5f24989cbc7fa5f27143d505))
* **xidlc:** fix rust type render ([30b4d09](https://github.com/xidl/xidl/commit/30b4d09cbee87d859f12f05c877c47cda97042db))
* **xidlc:** fix ts fetch ([e7d31df](https://github.com/xidl/xidl/commit/e7d31df1296291d274f2cb2e82997f5774b2bb42))
* **xidlc:** fix wasm build ([782df53](https://github.com/xidl/xidl/commit/782df53b8ba70cc40e08b0dcbf534b29a506e162))
* **xidlc:** fix wasm build ([16c4790](https://github.com/xidl/xidl/commit/16c4790254bd1dfa373a42367c07e8ed9467a26c))
* **xidlc:** map rust any to serde_json value ([7376020](https://github.com/xidl/xidl/commit/7376020ec717ccc6bd33588d5d27bee76e470592))


### Performance Improvements

* **xidlc:** faster format ([b2ff3c4](https://github.com/xidl/xidl/commit/b2ff3c43dc4973dd07cdaf6046786bb44027ed9d))

## [0.39.0](https://github.com/xidl/xidl/compare/v0.38.0...v0.39.0) (2026-04-08)


### Features

* add http hir layer ([0f3a307](https://github.com/xidl/xidl/commit/0f3a307537f96b9d0b9e56739098805c7c45f7fe))
* **http:** add body and flatten annotations ([8753d92](https://github.com/xidl/xidl/commit/8753d929c5ad9829e796700b39d781b1689f88ce))


### Bug Fixes

* **http:** wrap void out responses as objects ([b5ef38d](https://github.com/xidl/xidl/commit/b5ef38d454d941c18c98453bf5be492459c77922))
* remove gen-hir and gen-typed-ast feature ([c8a14d7](https://github.com/xidl/xidl/commit/c8a14d7719beb3a930a8c4ddf09a3e90d9ce2d31))
* **rust-axum:** fix template ([bac9a85](https://github.com/xidl/xidl/commit/bac9a85412b81c28e480edc70138dcc7a02d74b4))
* **rust:** qualify generated BTreeMap paths ([6ff8293](https://github.com/xidl/xidl/commit/6ff8293950da728bf5801e87ac2dce028cc2a861))

## [0.38.0](https://github.com/xidl/xidl/compare/v0.37.0...v0.38.0) (2026-04-08)


### Features

* **git:** add Cargo.lock ([87b3497](https://github.com/xidl/xidl/commit/87b3497fed31d92f57b0b1b603f75f87333392bd))


### Bug Fixes

* fix release problem ([c35f9ac](https://github.com/xidl/xidl/commit/c35f9ac0c3fff69d297d2f0508663e8736e07e97))

## [0.37.0](https://github.com/xidl/xidl/compare/v0.36.1...v0.37.0) (2026-04-07)


### Features

* **xidlc:** add python http generators and runtime ([1cdff5a](https://github.com/xidl/xidl/commit/1cdff5a0a8f1f9ce4d7dfda9cead47396a06efb4))

## [0.36.1](https://github.com/xidl/xidl/compare/v0.36.0...v0.36.1) (2026-04-07)


### Bug Fixes

* fix cargo publish ([60f576e](https://github.com/xidl/xidl/commit/60f576eae160676e2e2b25306de5c2708da4a968))

## [0.36.0](https://github.com/xidl/xidl/compare/v0.35.0...v0.36.0) (2026-04-07)


### Features

* **xildc-jsonrpc:** rename feature ([b5e3f9a](https://github.com/xidl/xidl/commit/b5e3f9a503153c4d0105bd6cdd7360f61f9236e8))

## [0.35.0](https://github.com/xidl/xidl/compare/v0.34.0...v0.35.0) (2026-04-04)


### Features

* make build faster ([908a6d7](https://github.com/xidl/xidl/commit/908a6d74bacdd7614a8fbea1551fad6a54163e84))

## [0.34.0](https://github.com/xidl/xidl/compare/v0.33.1...v0.34.0) (2026-04-03)


### Features

* **xidlc:** update format ([0695679](https://github.com/xidl/xidl/commit/0695679d823da801e0473c465b586d026701f21f))

## [0.33.1](https://github.com/xidl/xidl/compare/v0.33.0...v0.33.1) (2026-04-03)


### Bug Fixes

* **workflow:** ensure publish-release workflow runs on all events ([fd99ec7](https://github.com/xidl/xidl/commit/fd99ec79216bf49b31f6554adff2f7a0dd6a99f7))

## [0.33.0](https://github.com/xidl/xidl/compare/v0.32.0...v0.33.0) (2026-04-03)


### Features

* add CD workflow and improve release automation ([755d4be](https://github.com/xidl/xidl/commit/755d4be6068188b2763e6976643a61282d1a7d90))
* add Homebrew formula and improve Windows package support ([4ad1161](https://github.com/xidl/xidl/commit/4ad116103c09680e680b1ecaca848cec4e88f477))
* **rust-jsonrpc:** add conditional compilation for unsupported platforms ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **rust-jsonrpc:** optimize bidirectional method matching ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **workflow:** add dynamic release context resolution and manual dispatch support ([f9d0d24](https://github.com/xidl/xidl/commit/f9d0d24a02e5f4a0da40c6731673472673caa5b2))


### Bug Fixes

* **formula:** update xidlc.rb SHA256 checksum for v0.32.0 ([3b3c71c](https://github.com/xidl/xidl/commit/3b3c71c05a53b1875b03f0f115a5baebf4cda0df))
* **go-rest:** correct path parameter pattern replacement logic ([a27084c](https://github.com/xidl/xidl/commit/a27084ca0a162af4813fb1a5b85987869b172f75))
* **workflow:** enhance release metadata resolution with version parsing ([3b3c71c](https://github.com/xidl/xidl/commit/3b3c71c05a53b1875b03f0f115a5baebf4cda0df))

## [0.32.0](https://github.com/xidl/xidl/compare/v0.31.0...v0.32.0) (2026-04-03)


### Features

* **go-rest:** complete HTTP RFC support ([f9f7896](https://github.com/xidl/xidl/commit/f9f7896f63ae92d4cbfb6f0da96cec8f9c0f2aec))
* **pre-commit:** support pre-commit ([a82ca67](https://github.com/xidl/xidl/commit/a82ca6700b7b338133e126869629f12c033f5ae3))
* **xidlc:** add go and go-rest targets ([2c4d4c4](https://github.com/xidl/xidl/commit/2c4d4c4a28076127334da203d5d62ccd14553e64))


### Bug Fixes

* **go:** add delegated go test targets and ci coverage ([38acb67](https://github.com/xidl/xidl/commit/38acb671537301330bb3199eb6dfed22fc180305))
* **idl:** normalize oauth scopes for fmt and pre-commit ([76e3fbc](https://github.com/xidl/xidl/commit/76e3fbcc41cd5762d2beaff70399f355536146b8))

## [0.31.0](https://github.com/xidl/xidl/compare/v0.30.0...v0.31.0) (2026-03-31)


### Features

* add hy2 ([b1c0100](https://github.com/xidl/xidl/commit/b1c0100b80d7155804a3001a4e4d53dcd7d6b04b))
* add msgpack support ([bc5876c](https://github.com/xidl/xidl/commit/bc5876c5508844b06b5c6980161fa0e0c9a43051))
* **axum:** support pluggable http body codecs ([f2a8cdd](https://github.com/xidl/xidl/commit/f2a8cdd824cf4ae89fab8b66aa9f9db80e900878))

## [0.30.0](https://github.com/xidl/xidl/compare/v0.29.0...v0.30.0) (2026-03-21)


### Features

* **xidlc:** make fmt as a feature ([4ff313a](https://github.com/xidl/xidl/commit/4ff313a0ec01ce29f0a1a5b9d9a1b71c2e107345))


### Bug Fixes

* typo ([92ed86b](https://github.com/xidl/xidl/commit/92ed86b742fc9840337d0d75a4e5a9c7b94e9ed0))

## [0.29.0](https://github.com/xidl/xidl/compare/v0.28.0...v0.29.0) (2026-03-20)


### Features

* **xidl-rust-axum:** make reqwest as a optional dep ([743d5ad](https://github.com/xidl/xidl/commit/743d5ad917952d9b9a9c3ad9d3bdef115db0dc20))

## [0.28.0](https://github.com/xidl/xidl/compare/v0.27.0...v0.28.0) (2026-03-18)


### Features

* **axum:** impl auth for client ([9612ce5](https://github.com/xidl/xidl/commit/9612ce5cf4f37627e2c3bc4228d5c8863df2878e))

## [0.27.0](https://github.com/xidl/xidl/compare/v0.26.0...v0.27.0) (2026-03-18)


### Features

* **axum:** impl http_bearer ([275c165](https://github.com/xidl/xidl/commit/275c165bb043091c06fb89dbd6c50a0f8aa276d6))

## [0.26.0](https://github.com/xidl/xidl/compare/v0.25.0...v0.26.0) (2026-03-17)


### Features

* **axu,:** impl basic-auth ([43ecad3](https://github.com/xidl/xidl/commit/43ecad3abf01d594fc0a51a9f2aec0779522c029))
* **axum:** update attribute generate ([1961984](https://github.com/xidl/xidl/commit/1961984053cb454378057d50fab5358ece270081))

## [0.25.0](https://github.com/xidl/xidl/compare/v0.24.0...v0.25.0) (2026-03-16)


### Features

* using underscore instead of - in annotation ([01e53bf](https://github.com/xidl/xidl/commit/01e53bf5e2310bd26e49aea6ab53c2b88f175104))
* **xidl:** set lints.workspace=true ([2e63ba4](https://github.com/xidl/xidl/commit/2e63ba408b6a0d84887f3a8f09e887c13e39cadb))

## [0.24.0](https://github.com/xidl/xidl/compare/v0.23.0...v0.24.0) (2026-03-16)


### Features

* **openapi:** auto select openapi version ([6aad440](https://github.com/xidl/xidl/commit/6aad4400f232db1e2017983e7ccdb07015673e30))

## [0.23.0](https://github.com/xidl/xidl/compare/v0.22.0...v0.23.0) (2026-03-15)


### Features

* **xidlc:** support [@rust](https://github.com/rust) annotation ([a8f10d6](https://github.com/xidl/xidl/commit/a8f10d64b2ad6770543c7ea96c37a8086f280bfd))
* **xidlc:** support HIR include expansion ([42a715b](https://github.com/xidl/xidl/commit/42a715bfe6a2b7021f8392aca4e352a69efbb6ef))

## [0.22.0](https://github.com/xidl/xidl/compare/v0.21.0...v0.22.0) (2026-03-14)


### Features

* Add OpenAPI 3.2 HTTP stream support ([7d285d6](https://github.com/xidl/xidl/commit/7d285d69b853102c66021cdf96a2a8346fe3f592))
* align unary http and http security mapping ([c0dad07](https://github.com/xidl/xidl/commit/c0dad0728aa735267ecc3715d4ef04871588c015))

## [0.21.0](https://github.com/xidl/xidl/compare/v0.20.0...v0.21.0) (2026-03-13)


### Features

* **jsonrpc:** add ipc transport for plugins ([5d9cfdb](https://github.com/xidl/xidl/commit/5d9cfdbf026474594f71303ab2a8c22a5ab9b9d0))
* **jsonrpc:** expose bound server endpoint ([a933984](https://github.com/xidl/xidl/commit/a9339842cf72655d2670dc6a5eafcd4ad95840e5))


### Bug Fixes

* **xidlc:** map rust any to serde_json value ([7376020](https://github.com/xidl/xidl/commit/7376020ec717ccc6bd33588d5d27bee76e470592))

## [0.20.0](https://github.com/xidl/xidl/compare/v0.19.0...v0.20.0) (2026-03-13)


### Features

* **axum:** render reqwest error by debug ([6b6908f](https://github.com/xidl/xidl/commit/6b6908f4ed0a344095830bf7ebb9515fd4fe59aa))

## [0.19.0](https://github.com/xidl/xidl/compare/v0.18.0...v0.19.0) (2026-03-13)


### Features

* add doc comment to annotation ([bb1e191](https://github.com/xidl/xidl/commit/bb1e1918d14cc56657777bdf598a031cce209af6))
* **xidlc:** render doc ([c4d32e7](https://github.com/xidl/xidl/commit/c4d32e7b69803108c192496a768b3315b20f91af))
* **xidlc:** render doc ([eab28b5](https://github.com/xidl/xidl/commit/eab28b5968b17b122a4a4b6277e79a16f59a25f4))

## [0.18.0](https://github.com/xidl/xidl/compare/v0.17.0...v0.18.0) (2026-03-12)


### Features

* apply [@name](https://github.com/name) to rust struct fields ([ebe42f0](https://github.com/xidl/xidl/commit/ebe42f0efb5ec2a5a69a8576738a0ef66218ba43))
* support [@name](https://github.com/name) on enum members ([909595f](https://github.com/xidl/xidl/commit/909595fae6b450a2b12d25ef74977374d0f14c2d))

## [0.17.0](https://github.com/xidl/xidl/compare/v0.16.0...v0.17.0) (2026-03-12)


### Features

* add [@name](https://github.com/name) field rename ([bef6497](https://github.com/xidl/xidl/commit/bef64973f20eae1f42eab971757a2049ba0945ec))

## [0.16.0](https://github.com/xidl/xidl/compare/v0.15.0...v0.16.0) (2026-03-12)


### Features

* add [@http](https://github.com/http)(rename) for field serialization ([d7cb3c8](https://github.com/xidl/xidl/commit/d7cb3c88783f0b115d585a4fd7acbb8120b21697))

## [0.15.0](https://github.com/xidl/xidl/compare/v0.14.0...v0.15.0) (2026-03-12)


### Features

* add [@cookie](https://github.com/cookie) support for http generators ([50c005b](https://github.com/xidl/xidl/commit/50c005b073f425a0fb24e5ec6d767445cac0443b))

## [0.14.0](https://github.com/xidl/xidl/compare/v0.13.0...v0.14.0) (2026-03-12)


### Features

* **http:** add [@header](https://github.com/header) support ([d01783a](https://github.com/xidl/xidl/commit/d01783a207c5a90cc2ea442670925e85d3755c5e))

## [0.13.0](https://github.com/xidl/xidl/compare/v0.12.0...v0.13.0) (2026-03-11)


### Features

* remove playground ([bae5e45](https://github.com/xidl/xidl/commit/bae5e451a4947b22ca585a6f5d850248bc048774))
* **xidlc:** dont't generate ts and openapi in axum ([b86dd13](https://github.com/xidl/xidl/commit/b86dd131dbc54fdff55eaf1680385d480d1b3de0))
* **xidlc:** generate service code by default ([d03ccf6](https://github.com/xidl/xidl/commit/d03ccf6f00467ccba89be6d5bf6645d7f73424c3))

## [0.12.0](https://github.com/xidl/xidl/compare/v0.11.0...v0.12.0) (2026-03-10)


### Features

* **ts:** generate interface for ts ([0e0ae3a](https://github.com/xidl/xidl/commit/0e0ae3a63924dc06b6698452e4bcd38edb398567))

## [0.11.0](https://github.com/xidl/xidl/compare/v0.10.0...v0.11.0) (2026-03-10)


### Features

* **openapi:** support pragma xidlc service ([649f542](https://github.com/xidl/xidl/commit/649f542540940b2ccf013e72c2613243ce54f87c))

## [0.10.0](https://github.com/xidl/xidl/compare/v0.9.0...v0.10.0) (2026-03-09)


### Features

* **playground:** add openapi and openrpc ([e68971d](https://github.com/xidl/xidl/commit/e68971d23779c0beafa29ce2a7356050a14d5ec6))
* **xidl-rust-axum:** update error model ([5712624](https://github.com/xidl/xidl/commit/5712624aa0e8046affd7e39aeacf6775b3e9ea2d))

## [0.9.0](https://github.com/xidl/xidl/compare/v0.8.0...v0.9.0) (2026-03-09)


### Features

* **axum:** update axum by http ([00b3743](https://github.com/xidl/xidl/commit/00b374309a8c4693c027f689a39df36604ec8360))
* bump tree-sitter-idl to 3.16.0 and add annotation support for param_dcl ([60e19ac](https://github.com/xidl/xidl/commit/60e19acb934ffc61fac1f90ad2d5049460c41e37))
* complete http stream and jsonrpc stream ([41373f6](https://github.com/xidl/xidl/commit/41373f6d6fa2e6cb78b4590bfcdc6e45202f1099))
* **http:** document the in, out and inout ([70c2d1b](https://github.com/xidl/xidl/commit/70c2d1bdde2b16622f0ad573bcdf5a0f25555e06))
* **jsonpc:** add tcp and inproc support ([e6c7b93](https://github.com/xidl/xidl/commit/e6c7b93a32f5aaed976d38622ae402c202fd5352))
* **jsonrpc:** add quic support ([c07e660](https://github.com/xidl/xidl/commit/c07e6605b2cc533eee5005345650048217fc08cb))
* **jsonrpc:** add ws, wss, tls support ([fc98fdc](https://github.com/xidl/xidl/commit/fc98fdc7781bdff5f3f9dfeabe61764457109096))
* **jsonrpc:** impl stream for jsonrpc ([df85fe5](https://github.com/xidl/xidl/commit/df85fe5ee9b241047eb265352eb4162cbe5bb6fa))
* **jsonrpc:** update rust_jsonrpc ([66f9448](https://github.com/xidl/xidl/commit/66f9448bb73261edef8bc487a78f9cf9fb6b5d0c))
* **openapi:** unify return code ([609dc5a](https://github.com/xidl/xidl/commit/609dc5a7d6ca3e85d8022ab6cfef676954b27b16))
* **rust-axum:** impl bidi_stream ([9b1f1b8](https://github.com/xidl/xidl/commit/9b1f1b883af35016161d33bcd59753a0d319a701))
* **stream:** using writer on the client_side ([9dc3d1b](https://github.com/xidl/xidl/commit/9dc3d1bd8c53a76194f5ed19c7ff562c1dd18b9e))
* support query param ([bd818c4](https://github.com/xidl/xidl/commit/bd818c4b003776515ba46aef0eb400124ffc1170))
* **xidl-build:** allow set openapi and openrpc output file name ([59eb939](https://github.com/xidl/xidl/commit/59eb939598f1f1693d2658d50a6c22f1c9ba4aba))
* **xidlc:** add is_optional to member ([bbc7ed2](https://github.com/xidl/xidl/commit/bbc7ed2cd7ed71a459bdcf0fd6d10edd08a55d91))
* **xidlc:** add openrpc support ([081a4a9](https://github.com/xidl/xidl/commit/081a4a9bd8c2430b3e502db792705593d8fe4142))
* **xidlc:** add support for [@optional](https://github.com/optional) ([85a75dd](https://github.com/xidl/xidl/commit/85a75dd493d45f84c027080279d63eb494943d12))
* **xidlc:** align with rfc ([368f742](https://github.com/xidl/xidl/commit/368f742b8b70531f2f2b7b5ea9c771b4bee048a8))
* **xidlc:** impl stream for axum, ts and openapi ([64e8344](https://github.com/xidl/xidl/commit/64e834425ae2fb6df650d333a840b848d1879f12))
* **xidlc:** remove axum nested object ([7c8262b](https://github.com/xidl/xidl/commit/7c8262bf6ad3e8b8b442cac6022476b5b5112335))
* **xidlc:** update axum and ts by http rfc ([8f1fcd7](https://github.com/xidl/xidl/commit/8f1fcd744f773ff6128497648aa57a3ebf235fd5))
* **xidl:** support catch all path ([96a6540](https://github.com/xidl/xidl/commit/96a654000f9a39b111867dc325664f073e0ad074))
* **xidl:** update ts by http ([71a388b](https://github.com/xidl/xidl/commit/71a388b74803f83c7f642f7c14ec0fab14e6bba5))


### Bug Fixes

* **jsonrpc:** fix jsonrpc ([ffa6aac](https://github.com/xidl/xidl/commit/ffa6aacf5df26ccd1d39f72f1a38d250178f8220))
* **openrpc:** fix openrpc generate ([359c228](https://github.com/xidl/xidl/commit/359c2289dac54ae9637f176d031c8cca20fdf7a4))
* **xidlc:** fix axum generate ([da8c841](https://github.com/xidl/xidl/commit/da8c841e8350e4705b68a5a08c8916a689e7e69b))
* **xidlc:** fix wasm build ([782df53](https://github.com/xidl/xidl/commit/782df53b8ba70cc40e08b0dcbf534b29a506e162))

## [0.8.0](https://github.com/xidl/xidl/compare/v0.7.0...v0.8.0) (2026-03-04)


### Features

* **xidlc:** rust enum support serialize ([eb34505](https://github.com/xidl/xidl/commit/eb34505cb0aaabdcc5700864c4226c1978b8c6d7))

## [0.7.0](https://github.com/xidl/xidl/compare/v0.6.0...v0.7.0) (2026-03-04)


### Features

* bump tree-sitter-idl to 3.15.0 ([f2617c8](https://github.com/xidl/xidl/commit/f2617c85a197e7a7c648aa8f42f2505d05ac11f7))
* **xidl:** support template type ([3e27c1a](https://github.com/xidl/xidl/commit/3e27c1a5d67c40135f80377866bd6115a514039e))

## [0.6.0](https://github.com/xidl/xidl/compare/v0.5.0...v0.6.0) (2026-03-04)


### Features

* **rust-axum:** add serve_with_listener ([a3bdcbc](https://github.com/xidl/xidl/commit/a3bdcbc4765394b5c962df46a8b4564972361d43))
* **rust-axum:** make error as const ([fc5146b](https://github.com/xidl/xidl/commit/fc5146b9556a3fce5271b08e2103db3bbf5e87be))

## [0.5.0](https://github.com/xidl/xidl/compare/v0.4.0...v0.5.0) (2026-03-04)


### Features

* **rust-axum:** add more error method ([63c0c15](https://github.com/xidl/xidl/commit/63c0c15afd2cc774bdbc47ab2003bb4b8f7619c6))

## [0.4.0](https://github.com/xidl/xidl/compare/v0.3.0...v0.4.0) (2026-03-04)


### Features

* **xidl-build:** add `with_` prefix for all method ([27979da](https://github.com/xidl/xidl/commit/27979dafc79d943d74cb5afdad5377827987ae7d))

## [0.3.0](https://github.com/xidl/xidl/compare/v0.2.1...v0.3.0) (2026-03-03)


### Features

* **xidlc:** make openapi as a single language ([6a6f2a1](https://github.com/xidl/xidl/commit/6a6f2a19ec8a7af5d67be9c323a811bbcb47c0f6))

## [0.2.1](https://github.com/xidl/xidl/compare/v0.2.0...v0.2.1) (2026-03-03)


### Bug Fixes

* **xidl-build:** fix method name ([87509cf](https://github.com/xidl/xidl/commit/87509cf16b4a952738fa47f20bc0e62535eae3ac))

## [0.2.0](https://github.com/xidl/xidl/compare/v0.1.0...v0.2.0) (2026-03-03)


### Features

* add annotation ([ea21e45](https://github.com/xidl/xidl/commit/ea21e45dcfaf75ba948e192c7a59e7fc95f0f3e5))
* add bitmask, typedec_dcl and etc ([728bdb6](https://github.com/xidl/xidl/commit/728bdb6dad74b379bc7bf6a4ea1eb0e4bd091960))
* add debug ([8437275](https://github.com/xidl/xidl/commit/843727591ca6560a2c7874fe8f688780745cb718))
* add docusaurus ([72209dc](https://github.com/xidl/xidl/commit/72209dc1f991ebbdb6f6590251ede8bf5e676b17))
* add hir ([0a02a17](https://github.com/xidl/xidl/commit/0a02a17854199f3cffcbc2e75540f3d202c8bf2b))
* add idlc ([1f1aab4](https://github.com/xidl/xidl/commit/1f1aab484742a802e92defad8c99f1d97714adb1))
* add interface ([252651a](https://github.com/xidl/xidl/commit/252651ad1052b46994bb9c46c3fb33ec0fc940ab))
* add more support for struct ([c9c21b9](https://github.com/xidl/xidl/commit/c9c21b98ae1f3bdf4f34d27ccbaac472f612141b))
* add more test ([4426f8d](https://github.com/xidl/xidl/commit/4426f8d9246d2108738cf1e205fb1f2ad74a2dec))
* add playground ([b8663e7](https://github.com/xidl/xidl/commit/b8663e70561c51adfabab5d02162951261872076))
* add typeobject ([fab7a49](https://github.com/xidl/xidl/commit/fab7a49cec8bd943f11cf1c4c891b1e36349ad58))
* add typescript support, fix code gen ([5d65d11](https://github.com/xidl/xidl/commit/5d65d1163a812d12ba1b34ff3e65b07fd27e4603))
* add union and enum ([1a29dd4](https://github.com/xidl/xidl/commit/1a29dd45f63795e482c3e036a4aad1841d6dc9d4))
* add xidl-build ([01dfe3e](https://github.com/xidl/xidl/commit/01dfe3ea7b68e3550e7f20930833f4b9ef72fe52))
* add xidl-rust-axum ([2b40482](https://github.com/xidl/xidl/commit/2b40482a21c942f2a5f9aa605dcf736c483e3509))
* add xidl-xcdr ([4f0394c](https://github.com/xidl/xidl/commit/4f0394cc4ea62a33d80bfaaa1165bc93b36a81d5))
* add xidlc-example ([5ae00fb](https://github.com/xidl/xidl/commit/5ae00fbdc9e6160c83ecd34f9f94b1ba09f71020))
* **axum:** allow skip_client or skip_server ([6010a45](https://github.com/xidl/xidl/commit/6010a454eb387107b803b2bf0d429c6dd30758a7))
* **axum:** update axum ([e7aef77](https://github.com/xidl/xidl/commit/e7aef774dec984dcb23029fc8b2284ac9625346c))
* **bitfield:** support bitfield with fields ([ed00dde](https://github.com/xidl/xidl/commit/ed00dde9a0daeaf506f92b605f38fe68784be2bf))
* bump reqwest to 0.13.2 ([c15dff2](https://github.com/xidl/xidl/commit/c15dff2d4547547129d48ee8f03d577bc8e4ff36))
* **cargo:** strip file for release ([20c5992](https://github.com/xidl/xidl/commit/20c59920bff114ec31e20f07a7ed15b54810b539))
* **cli:** rename fmt language ([98084f2](https://github.com/xidl/xidl/commit/98084f282fd230ab2446bfd4c1e82d222f932b18))
* complete const_dcl ([f8d56d5](https://github.com/xidl/xidl/commit/f8d56d5eddb918cd0fe5717e09eca2cb1d7dc96c))
* **const:** impl const ([6493166](https://github.com/xidl/xidl/commit/64931661db2f2f49663ec5d353cc5270de96fa78))
* **const:** support socped name ([65cdae2](https://github.com/xidl/xidl/commit/65cdae2b109807122ae8f2206604710c1b696d44))
* **diagnostic:** refactor code ([cb97ff1](https://github.com/xidl/xidl/commit/cb97ff1b4932b35889edeee13d320edbae98dd9b))
* first commit ([c37354d](https://github.com/xidl/xidl/commit/c37354d2444e16807695ed586b818b5a8b8a0975))
* **fmt:** donot allow format error when test ([6e71122](https://github.com/xidl/xidl/commit/6e7112272b432467cb16e5a20116df843daaebf2))
* **hir:** add const and interface mappings ([3a026c7](https://github.com/xidl/xidl/commit/3a026c731a4405ccf79cbc13c71f06d9121e23dc))
* **hir:** add field_id ([2873091](https://github.com/xidl/xidl/commit/28730911be8a0341fc0aa8813632fff95b7c68c3))
* **hir:** add more implement ([8018bd2](https://github.com/xidl/xidl/commit/8018bd26483fc385974adddd3fe8577e3f0e5684))
* **hir:** add template type specs ([7ae9a09](https://github.com/xidl/xidl/commit/7ae9a09f4de88eee060ba167ce3194b2a855543e))
* **hir:** add union and bitset conversions ([eb3d1e1](https://github.com/xidl/xidl/commit/eb3d1e1235c9843b4f0d246dcf442a3ef70625ec))
* **hir:** support interface ([42371ca](https://github.com/xidl/xidl/commit/42371cacea4f7d872aee42c0fa799ef86067f5fb))
* **hir:** types don't relay on typed_ast ([06e760a](https://github.com/xidl/xidl/commit/06e760a9d121237cf68bba3f1ab3e8eb9ad34741))
* **idlc:** add bitmask, bitset and union ([4b52183](https://github.com/xidl/xidl/commit/4b52183d93992752d4940e634d1172547475d7fc))
* **idlc:** add rust module support ([07680b0](https://github.com/xidl/xidl/commit/07680b011a5ed464e3e712b7bbd1faa33dada2a8))
* **idlc:** add support for more serialize format ([0d92a56](https://github.com/xidl/xidl/commit/0d92a564b29fae5ce1033829889aa623afdaa590))
* **idlc:** better serialize ([f15a2c8](https://github.com/xidl/xidl/commit/f15a2c87715219ff09bb848611d85c8461698739))
* impl some idlc ([cf5407b](https://github.com/xidl/xidl/commit/cf5407b34f23337d119ce81090481315f39073e1))
* **jsonrpc:** add example ([fad7c8b](https://github.com/xidl/xidl/commit/fad7c8b48d85c93e274047853bc7b54e56c643df))
* **jsonrpc:** make jsonrpc tokio as optional ([17e1227](https://github.com/xidl/xidl/commit/17e1227c0c757d165e16ad4bf2eacfbf7309ed79))
* **jsonrpc:** refactor code ([81f7ec8](https://github.com/xidl/xidl/commit/81f7ec80f314e004351ba6ec026edf65151b19c0))
* **jsonrpc:** update jsonrpc ([6528364](https://github.com/xidl/xidl/commit/65283641166bf93a19d98a26072601b02d4e83b9))
* **jsonrpc:** update jsonrpc ([812ed3e](https://github.com/xidl/xidl/commit/812ed3e472d5d6bbca6327e0ce4d0777e5dff291))
* **jsonrpc:** using async_trait ([f961489](https://github.com/xidl/xidl/commit/f9614893ad503f26853d638932c05b0fbd8c15de))
* **jsonrpc:** using enum instead i64 ([22fcd82](https://github.com/xidl/xidl/commit/22fcd827684afbe24e4717f918329a94c3118137))
* make it works ([faff2bc](https://github.com/xidl/xidl/commit/faff2bcd1c1035f85a7d999159d86929906ad65e))
* parse some base-types ([9c60a96](https://github.com/xidl/xidl/commit/9c60a96f79aa16c934068800674a23362e281ca2))
* **parser:** add serialize, deserialize for typed_ast ([2e412b0](https://github.com/xidl/xidl/commit/2e412b0b3aa66b739b1d2c4ed6620ac347294111))
* **parser:** handle error node ([dff08b8](https://github.com/xidl/xidl/commit/dff08b8a5611b706587441488e458c0bfba1519d))
* **parser:** support extend_annonation ([af73f64](https://github.com/xidl/xidl/commit/af73f641fe3bd59783a46d7baa6677ce9e10fd51))
* pass the first test ([fa634bc](https://github.com/xidl/xidl/commit/fa634bcf5cc0624e9bd65b4fd6df1c4829f3645b))
* **playground:** add format ([49c35e8](https://github.com/xidl/xidl/commit/49c35e8f46c9431058056e66c2ea0d0de48a282d))
* **playground:** add share button ([541770b](https://github.com/xidl/xidl/commit/541770badfc2cef7078075f13d6cd6cf9e5e2de9))
* **playground:** support share code ([1202df5](https://github.com/xidl/xidl/commit/1202df548df044f5d20fb0ab237b15473764872c))
* **playground:** update playground ([3b2e729](https://github.com/xidl/xidl/commit/3b2e729eaff62080762d5534fb92bad930fee53a))
* **rust-axum:** support axum ([09c3522](https://github.com/xidl/xidl/commit/09c352289a0517e67c33c44d41f2e4688a8df801))
* **rust:** bumpd rust to 1.92 ([66d8010](https://github.com/xidl/xidl/commit/66d8010ade0169509570907ee1777d989f90fa80))
* **rust:** remove typeobject ([e43969c](https://github.com/xidl/xidl/commit/e43969c7f72eff046c7d1853bf2e65c6c46ff857))
* **rust:** support [@derive](https://github.com/derive) ([5992faf](https://github.com/xidl/xidl/commit/5992faf4141363474c31678b245e2febb5d3928a))
* **scoped_name:** handle is_root ([58a7bcb](https://github.com/xidl/xidl/commit/58a7bcbce6459fdb3b2516728539f03e5173a98e))
* support #pragma xidlc package and #pragma xidlc version ([fa3d418](https://github.com/xidl/xidl/commit/fa3d418921f03f647bd97ead0d75043a948a6c85))
* support float ([49e42c5](https://github.com/xidl/xidl/commit/49e42c57ed2509a66f6dda140cdb5ab01b8f3ce0))
* support more type ([5c42ffc](https://github.com/xidl/xidl/commit/5c42ffc8e71e70d632d890163dcf2a5f3a6a4225))
* support more type ([686e0bc](https://github.com/xidl/xidl/commit/686e0bcbaefece57a56c14511f5836b35b7199f4))
* support more type ([e16119c](https://github.com/xidl/xidl/commit/e16119c2022a66d21405ec67bae2e2238c98b538))
* **typed_ast:** expand corpus coverage ([4236784](https://github.com/xidl/xidl/commit/42367849edbe1e5fbd26d915050a8510fd2c3eaf))
* **union:** add union member support ([eb013cb](https://github.com/xidl/xidl/commit/eb013cb703ce75fc0df4e7f10da3e0b6b7c62352))
* x ([1db79eb](https://github.com/xidl/xidl/commit/1db79ebb11387b2a8eb69a1948d3310e49b506ae))
* **xcdr:** add delimited cdr ([521219f](https://github.com/xidl/xidl/commit/521219f385d7681154ca830d5867dd613811cc54))
* **xcdr:** add plain_cdr2 ([491c4cc](https://github.com/xidl/xidl/commit/491c4cc7e4a96241384f07fbe38511de3b7b3fbc))
* **xcdr:** add plain-cdr ([ef5a65b](https://github.com/xidl/xidl/commit/ef5a65bebd142442b7523429991a277a1dedf729))
* **xcdr:** add plcdr ([7084e64](https://github.com/xidl/xidl/commit/7084e641554e7f62f24619f796cd4e4acae11737))
* **xcdr:** add plcdr2 ([4bd18f5](https://github.com/xidl/xidl/commit/4bd18f5f94133f9306ed2c197307aa8b1c76df35))
* **xcdr:** add xcdr-plcdr ([a5f03ed](https://github.com/xidl/xidl/commit/a5f03eda2aa0e27fb23d07fc9f3c3c23e3d539a1))
* **xcdr:** better ffi ([5e5e301](https://github.com/xidl/xidl/commit/5e5e301ea27dde57dd68d568a50fa104671ae26b))
* **xcdr:** calc EMHEADER ([766df12](https://github.com/xidl/xidl/commit/766df120ca653dfcf206168a5c2e7bde389d2375))
* **xcdr:** reimpl xcdr ([48c063a](https://github.com/xidl/xidl/commit/48c063a58a06f8002f1299094496513c28f42548))
* **xidl:** add more methods ([ddd54f2](https://github.com/xidl/xidl/commit/ddd54f249238c6c9251cd5b702127b9494898b57))
* **xidl:** add parser attribute ([73ef095](https://github.com/xidl/xidl/commit/73ef0954513d3faf29ad32dda791e2be24b9ab00))
* **xidlc:** add artifact ([139031e](https://github.com/xidl/xidl/commit/139031ede4b2ad1058159e765b5190031fe35b68))
* **xidlc:** add code highlight ([417f3ef](https://github.com/xidl/xidl/commit/417f3efff0b09d040300c6c5c1d1fb6238ea5e07))
* **xidlc:** add cpp formatter ([f495752](https://github.com/xidl/xidl/commit/f495752c2a430ed4eced0fff5c0b4d3199fb8b2c))
* **xidlc:** add cpp serialize ([274cb44](https://github.com/xidl/xidl/commit/274cb44406e1e6b383db3d3747d2424e5c2984c9))
* **xidlc:** add deser code gen ([fe2920a](https://github.com/xidl/xidl/commit/fe2920abaefc9f0f2ee19fe4616c82d21d620a84))
* **xidlc:** add diagnosic module ([c78463a](https://github.com/xidl/xidl/commit/c78463a34d344ed0ba39d32fa53c0d9c57f20369))
* **xidlc:** add dry-run ([529292f](https://github.com/xidl/xidl/commit/529292f79f09121927f7883596aeed450a00e664))
* **xidlc:** add file header ([19cf730](https://github.com/xidl/xidl/commit/19cf730a9852bd2b9e322a8f8e145c5b70f4b6b0))
* **xidlc:** add format ([28d55e4](https://github.com/xidl/xidl/commit/28d55e4744cb25cc5cba22d230c614b001ee2ca4))
* **xidlc:** add get_engine_version method ([567986d](https://github.com/xidl/xidl/commit/567986d7ca0e20a4259fec1ca6405b86fea6ed2c))
* **xidlc:** add inplace for format ([9b08504](https://github.com/xidl/xidl/commit/9b0850496a03dcf7e52491ca647676f9357b454c))
* **xidlc:** add interface support ([7899205](https://github.com/xidl/xidl/commit/789920562b0e1e5f2a7e6eecb33cfd4073f7f089))
* **xidlc:** add jinja formatter ([8808e6e](https://github.com/xidl/xidl/commit/8808e6e29b64a1e54bde3bd0f4aa01bc0ee31b17))
* **xidlc:** add jsonrpc_full ([f2fe6b7](https://github.com/xidl/xidl/commit/f2fe6b7245aa21b9467050841f964917e8601983))
* **xidlc:** add log ([a52b475](https://github.com/xidl/xidl/commit/a52b475a76731db697cd774dde4aa61ae1629100))
* **xidlc:** add more diagnostic ([f49b13e](https://github.com/xidl/xidl/commit/f49b13ea0de1d12f0771890ad9a5343657600abe))
* **xidlc:** add rust format filter ([25bff71](https://github.com/xidl/xidl/commit/25bff718b0450790ee21982bd1bf9353435a9372))
* **xidlc:** add rust_jsonrpc ([b445719](https://github.com/xidl/xidl/commit/b4457195d45ce5ec71a462b60aadeca816185d75))
* **xidlc:** add spec template ([a85b656](https://github.com/xidl/xidl/commit/a85b6566a8d7341ae71b49b617a9cbac609c6965))
* **xidlc:** add support for rust ([0bafa1b](https://github.com/xidl/xidl/commit/0bafa1bd568b6322acb08071faa9230a8f9a0131))
* **xidlc:** add typed_ast_gen ([a630d38](https://github.com/xidl/xidl/commit/a630d389aae0b040ac31ac936422e0a22bfe78a1))
* **xidlc:** add typescript formatter ([838a3ce](https://github.com/xidl/xidl/commit/838a3ce2dade19d6507d932aef42f933e47bc47f))
* **xidlc:** allow generate hir ([7af7d76](https://github.com/xidl/xidl/commit/7af7d76717cc258f96d2d82ff929d2adbc270019))
* **xidlc:** allow skip serialize and deserialize in rust ([b31a956](https://github.com/xidl/xidl/commit/b31a9561778d98874993ee31f59b21652d0433ff))
* **xidlc:** better attribute ([b19d093](https://github.com/xidl/xidl/commit/b19d0933e7ebe3a6911f935c8d0352f335bbbfae))
* **xidlc:** better cli ([ab3e81d](https://github.com/xidl/xidl/commit/ab3e81db5f36e03663dc364d7674b7dcec4eb7cc))
* **xidlc:** better diagnosic ([5953bfd](https://github.com/xidl/xidl/commit/5953bfd021e30c8c2bdabcc5ca1a813dcfa65fc1))
* **xidlc:** better support for rust ([10fd91b](https://github.com/xidl/xidl/commit/10fd91baef24773ef808b0f5105fbacf71ba3fde))
* **xidlc:** diagnostic support filename ([b94d430](https://github.com/xidl/xidl/commit/b94d430f863cf37a46992e5c2e4cc8938b3b0846))
* **xidlc:** eat the json_rpc food ([9e91e24](https://github.com/xidl/xidl/commit/9e91e240ab2893e1b7b3798fa737d551aa64d7b4))
* **xidlc:** integrated highlighting into diagnostics ([46abbea](https://github.com/xidl/xidl/commit/46abbea1591210f4485d2e44fc2bda5d9bab53f6))
* **xidlc:** make rust union safe ([282f494](https://github.com/xidl/xidl/commit/282f494d429f8d660c0bb8a52798c11240afc769))
* **xidlc:** panic when formate in test ([577c900](https://github.com/xidl/xidl/commit/577c900ad779471b6c57fd9e7a94a73964f5e7d7))
* **xidlc:** print help if non file is provided ([b6c6e8f](https://github.com/xidl/xidl/commit/b6c6e8f732b55bab585110311dd9239a6ecdd9e7))
* **xidlc:** read FINAL and APPENDABLE and etc ([cbb79ee](https://github.com/xidl/xidl/commit/cbb79ee78e838f66fef310263b1e271f0fe998dd))
* **xidlc:** refactor cli fmt ([f41ebe7](https://github.com/xidl/xidl/commit/f41ebe76b9df7ef38aa12e13784dc7498669ddda))
* **xidlc:** remove rust module attribute ([11cc7bc](https://github.com/xidl/xidl/commit/11cc7bce57ced8be94be2b4df1c5ad969ac51a31))
* **xidlc:** short cmdline ([8961f47](https://github.com/xidl/xidl/commit/8961f470abb2899225dbe285a405f2fffa8c7bfe))
* **xidlc:** support cpp ([46a2436](https://github.com/xidl/xidl/commit/46a2436340e4fffb7dfd892e06fbf4aab3350243))
* **xidlc:** switch to async ([1104255](https://github.com/xidl/xidl/commit/1104255463a4f1bcb77b91579f8014714b8b279d))
* **xidlc:** update ([f7cbdbc](https://github.com/xidl/xidl/commit/f7cbdbce906196f22a5537fba6d59a1e66ad4801))
* **xidlc:** update rust code ([2cd4429](https://github.com/xidl/xidl/commit/2cd4429dfdc28bbec9a4df886cc2364097da3289))
* **xidlc:** update template ([85ef6a1](https://github.com/xidl/xidl/commit/85ef6a1b9997aa6ecf2ce8f2d87a5cc39f1fe48e))
* **xidlc:** using full typepath in rust ([9b647a9](https://github.com/xidl/xidl/commit/9b647a9558266480f65c826c66cb2dbe658335ff))
* **xidlc:** using mempipe for builtin language ([e37956e](https://github.com/xidl/xidl/commit/e37956ea6b75890ef99a2ad9108c3034b9e7dca0))
* **xidlc:** using rpc ([5bd242e](https://github.com/xidl/xidl/commit/5bd242ed3dcda04264f9a34fa61d2b575459ab1e))
* **xidlc:** using the enable but not disable ([8be7465](https://github.com/xidl/xidl/commit/8be7465a22d8e713809f82a4ce4c92af626ec8a5))
* **xidl:** generate file.rs instead mod.rs ([4e6eee7](https://github.com/xidl/xidl/commit/4e6eee72677bbd106d6485b524d7a3b5dc530970))
* **xidl:** split into two files ([1f7cc2f](https://github.com/xidl/xidl/commit/1f7cc2f8603f8cc86fb645816d03ab911edc33f7))


### Bug Fixes

* **examples:** fix warnings ([1cc2b66](https://github.com/xidl/xidl/commit/1cc2b66db6b36e9d97f0b4c64f2bf8e82b671f9e))
* fix build problem ([1aede36](https://github.com/xidl/xidl/commit/1aede360162dfefcd79078f3df5808fb849b9770))
* fix some error ([9b25de5](https://github.com/xidl/xidl/commit/9b25de5dffc99224e998bfcc2d941386f28d2a1c))
* **fmt:** fix formate problem ([cdaaa6a](https://github.com/xidl/xidl/commit/cdaaa6a42651410f0a02839380cdaeae597ac354))
* **idlc:** fix rust gen ([c908ce8](https://github.com/xidl/xidl/commit/c908ce8ab0de5c448bb25c7e02b225f70d78ff7f))
* **idlc:** rename idlc to xidlc ([1139abe](https://github.com/xidl/xidl/commit/1139abe035497f624b76d9bf2b14d26f9e1cf088))
* **rust-axum:** fix generate ([6ddc701](https://github.com/xidl/xidl/commit/6ddc70162251ece354df22a02093c34312c2b796))
* **rust-wasm:** remove openssl dep ([76afac8](https://github.com/xidl/xidl/commit/76afac84647e861f8abe9bccece5645054a285a3))
* **scoped_name:** rename identifier to vec ([6eedc2e](https://github.com/xidl/xidl/commit/6eedc2ebfcc1e66ee146e2fee57d97ddeebec4fb))
* **typed_ast:** parse attr raises declarator ([2398cd4](https://github.com/xidl/xidl/commit/2398cd49ee154079ee799efd64dcfd10db21cf16))
* **wasm:** fix xidlc on wasm ([a5805b1](https://github.com/xidl/xidl/commit/a5805b14276b970f9eca127185c003d3343a9d94))
* **xidlc:** fix block for wasm ([2e0300d](https://github.com/xidl/xidl/commit/2e0300d807fb052dd641afdb44f71a89fa86f255))
* **xidlc:** fix build on windows ([26f39a6](https://github.com/xidl/xidl/commit/26f39a6e901bbd054ae919f2866065c2b11cc970))
* **xidlc:** fix c/cpp generate ([878f460](https://github.com/xidl/xidl/commit/878f460e5a86b3c96207a6d4554f39d803a129ed))
* **xidlc:** fix rust crate name ([90bc773](https://github.com/xidl/xidl/commit/90bc7732b2fe277a5f24989cbc7fa5f27143d505))
* **xidlc:** fix rust type render ([30b4d09](https://github.com/xidl/xidl/commit/30b4d09cbee87d859f12f05c877c47cda97042db))
* **xidlc:** fix ts fetch ([e7d31df](https://github.com/xidl/xidl/commit/e7d31df1296291d274f2cb2e82997f5774b2bb42))
* **xidlc:** fix wasm build ([16c4790](https://github.com/xidl/xidl/commit/16c4790254bd1dfa373a42367c07e8ed9467a26c))


### Performance Improvements

* **xidlc:** faster format ([b2ff3c4](https://github.com/xidl/xidl/commit/b2ff3c43dc4973dd07cdaf6046786bb44027ed9d))
