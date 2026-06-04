package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"strings"

	"github.com/gin-gonic/gin"
)

func formatOpt(v *string) string {
	if v == nil {
		return "None"
	}
	return fmt.Sprintf("Some(\"%s\")", *v)
}

func formatOptInt(v *uint32) string {
	if v == nil {
		return "None"
	}
	return fmt.Sprintf("Some(%d)", *v)
}

type MyE2EPathSever struct{}

func (s *MyE2EPathSever) OpWithPath(ctx context.Context, req *E2EPathSeverOpWithPathRequest) (*E2EPathSeverOpWithPathResponse, error) {
	return &E2EPathSeverOpWithPathResponse{Return: []string{req.Param1}}, nil
}

func (s *MyE2EPathSever) OpWithQuery(ctx context.Context, req *E2EPathSeverOpWithQueryRequest) (*E2EPathSeverOpWithQueryResponse, error) {
	return &E2EPathSeverOpWithQueryResponse{Return: []string{req.Param1, req.Q}}, nil
}

func (s *MyE2EPathSever) OpWithParams(ctx context.Context, req *E2EPathSeverOpWithParamsRequest) (*E2EPathSeverOpWithParamsResponse, error) {
	// Format body and a map to match the test assertion
	res := []string{req.PathName}
	res = append(res, req.Q...)
	res = append(res, fmt.Sprintf("%v", req.B))
	res = append(res, fmt.Sprintf("%v", req.A))
	return &E2EPathSeverOpWithParamsResponse{Return: res}, nil
}

func (s *MyE2EPathSever) OpWithQuery2(ctx context.Context, req *E2EPathSeverOpWithQuery2Request) (*E2EPathSeverOpWithQuery2Response, error) {
	return &E2EPathSeverOpWithQuery2Response{Return: req.All + ":" + req.Word + ":" + req.Q}, nil
}

type MyE2EHttpRouteAndBody struct{}

func (s *MyE2EHttpRouteAndBody) GetResource(ctx context.Context, req *E2EHttpRouteAndBodyGetResourceRequest) (*E2EHttpRouteAndBodyGetResourceResponse, error) {
	return &E2EHttpRouteAndBodyGetResourceResponse{Return: fmt.Sprintf("id:%s,lang:%s,trace:%s", req.ResourceId, formatOpt(req.Locale), req.TraceId)}, nil
}

func (s *MyE2EHttpRouteAndBody) GetFile(ctx context.Context, req *E2EHttpRouteAndBodyGetFileRequest) (*E2EHttpRouteAndBodyGetFileResponse, error) {
	filePath := req.FilePath
	if strings.HasPrefix(filePath, "/") {
		filePath = filePath[1:]
	}
	return &E2EHttpRouteAndBodyGetFileResponse{Return: fmt.Sprintf("file:%s,download:%t,version:%s", filePath, req.Download, formatOpt(req.Version))}, nil
}

func (s *MyE2EHttpRouteAndBody) CreateResource(ctx context.Context, req *E2EHttpRouteAndBodyCreateResourceRequest) (*E2EHttpRouteAndBodyCreateResourceResponse, error) {
	return &E2EHttpRouteAndBodyCreateResourceResponse{Return: req.ResourceBody}, nil
}

func (s *MyE2EHttpRouteAndBody) ReplaceResource(ctx context.Context, req *E2EHttpRouteAndBodyReplaceResourceRequest) (*E2EHttpRouteAndBodyReplaceResourceResponse, error) {
	return &E2EHttpRouteAndBodyReplaceResourceResponse{}, nil
}

func (s *MyE2EHttpRouteAndBody) PatchResource(ctx context.Context, req *E2EHttpRouteAndBodyPatchResourceRequest) (*E2EHttpRouteAndBodyPatchResourceResponse, error) {
	return &E2EHttpRouteAndBodyPatchResourceResponse{Return: req.Changes}, nil
}

func (s *MyE2EHttpRouteAndBody) DeleteResource(ctx context.Context, req *E2EHttpRouteAndBodyDeleteResourceRequest) (*E2EHttpRouteAndBodyDeleteResourceResponse, error) {
	return &E2EHttpRouteAndBodyDeleteResourceResponse{}, nil
}

func (s *MyE2EHttpRouteAndBody) ProbeResource(ctx context.Context, req *E2EHttpRouteAndBodyProbeResourceRequest) (*E2EHttpRouteAndBodyProbeResourceResponse, error) {
	return &E2EHttpRouteAndBodyProbeResourceResponse{}, nil
}

func (s *MyE2EHttpRouteAndBody) ResourceOptions(ctx context.Context, req *E2EHttpRouteAndBodyResourceOptionsRequest) (*E2EHttpRouteAndBodyResourceOptionsResponse, error) {
	return &E2EHttpRouteAndBodyResourceOptionsResponse{}, nil
}

func (s *MyE2EHttpRouteAndBody) GetMsgpackResource(ctx context.Context, req *E2EHttpRouteAndBodyGetMsgpackResourceRequest) (*E2EHttpRouteAndBodyGetMsgpackResourceResponse, error) {
	return &E2EHttpRouteAndBodyGetMsgpackResourceResponse{Return: StructHttpBody{Name: "msgpack"}, Revision: 1}, nil
}

func (s *MyE2EHttpRouteAndBody) DedupResource(ctx context.Context, req *E2EHttpRouteAndBodyDedupResourceRequest) (*E2EHttpRouteAndBodyDedupResourceResponse, error) {
	return &E2EHttpRouteAndBodyDedupResourceResponse{Return: req.Id + ":" + req.XTraceId}, nil
}

func (s *MyE2EHttpRouteAndBody) PreviewResource(ctx context.Context, req *E2EHttpRouteAndBodyPreviewResourceRequest) (*E2EHttpRouteAndBodyPreviewResourceResponse, error) {
	return &E2EHttpRouteAndBodyPreviewResourceResponse{Return: req.Resource}, nil
}

type MyE2EHttpSecurity struct{}

func (s *MyE2EHttpSecurity) GetSecureUser(ctx context.Context, req *E2EHttpSecurityGetSecureUserRequest) (*E2EHttpSecurityGetSecureUserResponse, error) {
	return &E2EHttpSecurityGetSecureUserResponse{Return: fmt.Sprintf("user:%s,lang:%s,trace:%s", req.UserId, formatOpt(req.Locale), req.TraceId)}, nil
}

func (s *MyE2EHttpSecurity) SearchSecureUser(ctx context.Context, req *E2EHttpSecuritySearchSecureUserRequest) (*E2EHttpSecuritySearchSecureUserResponse, error) {
	return &E2EHttpSecuritySearchSecureUserResponse{Return: fmt.Sprintf("keyword:%s,page:%s", req.Keyword, formatOptInt(req.Page))}, nil
}

func (s *MyE2EHttpSecurity) Healthz(ctx context.Context, req *E2EHttpSecurityHealthzRequest) (*E2EHttpSecurityHealthzResponse, error) {
	return &E2EHttpSecurityHealthzResponse{Return: "ok"}, nil
}

type MyE2ETypeServer struct {
	attr1 string
	attr2 []string
}

func (s *MyE2ETypeServer) GetAttributeTypeAttr1(ctx context.Context, req *E2ETypeServerGetAttributeTypeAttr1Request) (*E2ETypeServerGetAttributeTypeAttr1Response, error) {
	return &E2ETypeServerGetAttributeTypeAttr1Response{Return: s.attr1}, nil
}

func (s *MyE2ETypeServer) SetAttributeTypeAttr1(ctx context.Context, req *E2ETypeServerSetAttributeTypeAttr1Request) (*E2ETypeServerSetAttributeTypeAttr1Response, error) {
	s.attr1 = req.TypeAttr1
	return &E2ETypeServerSetAttributeTypeAttr1Response{}, nil
}

func (s *MyE2ETypeServer) GetAttributeTypeAttr2(ctx context.Context, req *E2ETypeServerGetAttributeTypeAttr2Request) (*E2ETypeServerGetAttributeTypeAttr2Response, error) {
	return &E2ETypeServerGetAttributeTypeAttr2Response{Return: s.attr2}, nil
}

func (s *MyE2ETypeServer) SimpleOp(ctx context.Context, req *E2ETypeServerSimpleOpRequest) (*E2ETypeServerSimpleOpResponse, error) {
	return &E2ETypeServerSimpleOpResponse{}, nil
}

func (s *MyE2ETypeServer) SimpleOpWithReturn1(ctx context.Context, req *E2ETypeServerSimpleOpWithReturn1Request) (*E2ETypeServerSimpleOpWithReturn1Response, error) {
	return &E2ETypeServerSimpleOpWithReturn1Response{Return: "simple_op_with_return1"}, nil
}

func (s *MyE2ETypeServer) SimpleOpWithReturn2(ctx context.Context, req *E2ETypeServerSimpleOpWithReturn2Request) (*E2ETypeServerSimpleOpWithReturn2Response, error) {
	return &E2ETypeServerSimpleOpWithReturn2Response{}, nil
}

func (s *MyE2ETypeServer) SimpleOpWithReturn3(ctx context.Context, req *E2ETypeServerSimpleOpWithReturn3Request) (*E2ETypeServerSimpleOpWithReturn3Response, error) {
	return &E2ETypeServerSimpleOpWithReturn3Response{Return: EnumSimple1V1}, nil
}

func (s *MyE2ETypeServer) SimpleOpWithReturn4(ctx context.Context, req *E2ETypeServerSimpleOpWithReturn4Request) (*E2ETypeServerSimpleOpWithReturn4Response, error) {
	return &E2ETypeServerSimpleOpWithReturn4Response{Return: StructEmpty{}}, nil
}

func (s *MyE2ETypeServer) SimpleOpWithReturn5(ctx context.Context, req *E2ETypeServerSimpleOpWithReturn5Request) (*E2ETypeServerSimpleOpWithReturn5Response, error) {
	return &E2ETypeServerSimpleOpWithReturn5Response{}, nil
}

func (s *MyE2ETypeServer) ReturnWithSequence1(ctx context.Context, req *E2ETypeServerReturnWithSequence1Request) (*E2ETypeServerReturnWithSequence1Response, error) {
	return &E2ETypeServerReturnWithSequence1Response{Return: []string{"s1", "s2"}}, nil
}

func (s *MyE2ETypeServer) ReturnWithSequence2(ctx context.Context, req *E2ETypeServerReturnWithSequence2Request) (*E2ETypeServerReturnWithSequence2Response, error) {
	return &E2ETypeServerReturnWithSequence2Response{}, nil
}

func (s *MyE2ETypeServer) ReturnWithSequence3(ctx context.Context, req *E2ETypeServerReturnWithSequence3Request) (*E2ETypeServerReturnWithSequence3Response, error) {
	return &E2ETypeServerReturnWithSequence3Response{Return: []EnumSimple1{EnumSimple1V1, EnumSimple1V2}}, nil
}

func (s *MyE2ETypeServer) ReturnWithSequence4(ctx context.Context, req *E2ETypeServerReturnWithSequence4Request) (*E2ETypeServerReturnWithSequence4Response, error) {
	return &E2ETypeServerReturnWithSequence4Response{Return: []StructEmpty{{}}}, nil
}

func (s *MyE2ETypeServer) ReturnWithSequence5(ctx context.Context, req *E2ETypeServerReturnWithSequence5Request) (*E2ETypeServerReturnWithSequence5Response, error) {
	return &E2ETypeServerReturnWithSequence5Response{}, nil
}

func (s *MyE2ETypeServer) ReturnWithMap(ctx context.Context, req *E2ETypeServerReturnWithMapRequest) (*E2ETypeServerReturnWithMapResponse, error) {
	return &E2ETypeServerReturnWithMapResponse{Return: map[string]uint8{"k1": 1}}, nil
}

func (s *MyE2ETypeServer) ReturnWithAny(ctx context.Context, req *E2ETypeServerReturnWithAnyRequest) (*E2ETypeServerReturnWithAnyResponse, error) {
	return &E2ETypeServerReturnWithAnyResponse{Return: map[string]any{"any": "value"}}, nil
}

func (s *MyE2ETypeServer) ReturnWithAnySequence(ctx context.Context, req *E2ETypeServerReturnWithAnySequenceRequest) (*E2ETypeServerReturnWithAnySequenceResponse, error) {
	return &E2ETypeServerReturnWithAnySequenceResponse{Return: []any{1, "two"}}, nil
}

func (s *MyE2ETypeServer) ReturnWithAnyMap(ctx context.Context, req *E2ETypeServerReturnWithAnyMapRequest) (*E2ETypeServerReturnWithAnyMapResponse, error) {
	return &E2ETypeServerReturnWithAnyMapResponse{Return: map[string]any{"k1": 1}}, nil
}

func (s *MyE2ETypeServer) ParameterOp(ctx context.Context, req *E2ETypeServerParameterOpRequest) (*E2ETypeServerParameterOpResponse, error) {
	return &E2ETypeServerParameterOpResponse{}, nil
}

func (s *MyE2ETypeServer) ParameterOp2(ctx context.Context, req *E2ETypeServerParameterOp2Request) (*E2ETypeServerParameterOp2Response, error) {
	return &E2ETypeServerParameterOp2Response{}, nil
}

func (s *MyE2ETypeServer) ParameterOp3(ctx context.Context, req *E2ETypeServerParameterOp3Request) (*E2ETypeServerParameterOp3Response, error) {
	return &E2ETypeServerParameterOp3Response{B: 3, C: []any{}}, nil
}

func (s *MyE2ETypeServer) ParameterOp4(ctx context.Context, req *E2ETypeServerParameterOp4Request) (*E2ETypeServerParameterOp4Response, error) {
	return &E2ETypeServerParameterOp4Response{A: "op4", B: 4, C: []any{}}, nil
}

func (s *MyE2ETypeServer) ParameterOp5(ctx context.Context, req *E2ETypeServerParameterOp5Request) (*E2ETypeServerParameterOp5Response, error) {
	return &E2ETypeServerParameterOp5Response{Return: []any{"op5"}, A: "op5", B: 5, C: []any{}}, nil
}

func (s *MyE2ETypeServer) ParameterOp6(ctx context.Context, req *E2ETypeServerParameterOp6Request) (*E2ETypeServerParameterOp6Response, error) {
	return &E2ETypeServerParameterOp6Response{Return: map[string]any{}, A: "op6", B: 6, C: []any{}}, nil
}

type MyE2EAttribute struct {
	attr1  string
	attr2  []string
	attr3  EnumEmpty
	attr4  EnumSimple1
	attr5  StructEmpty
	attr6  StructSimple
	attr61 UnionSimple
	attr7  []string
	attr8  []EnumEmpty
	attr9  []EnumSimple1
	attr10 []StructEmpty
	attr11 []StructSimple
	attr12 map[string]uint8
	attr13 any
	attr14 []any
	attr15 map[string]any
	attr16 string
}

func (s *MyE2EAttribute) GetAttributeAttr1(ctx context.Context, req *E2EAttributeGetAttributeAttr1Request) (*E2EAttributeGetAttributeAttr1Response, error) {
	return &E2EAttributeGetAttributeAttr1Response{Return: s.attr1}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr1(ctx context.Context, req *E2EAttributeSetAttributeAttr1Request) (*E2EAttributeSetAttributeAttr1Response, error) {
	s.attr1 = req.Attr1
	return &E2EAttributeSetAttributeAttr1Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr2(ctx context.Context, req *E2EAttributeGetAttributeAttr2Request) (*E2EAttributeGetAttributeAttr2Response, error) {
	return &E2EAttributeGetAttributeAttr2Response{Return: s.attr2}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr3(ctx context.Context, req *E2EAttributeGetAttributeAttr3Request) (*E2EAttributeGetAttributeAttr3Response, error) {
	return &E2EAttributeGetAttributeAttr3Response{Return: s.attr3}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr3(ctx context.Context, req *E2EAttributeSetAttributeAttr3Request) (*E2EAttributeSetAttributeAttr3Response, error) {
	s.attr3 = req.Attr3
	return &E2EAttributeSetAttributeAttr3Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr4(ctx context.Context, req *E2EAttributeGetAttributeAttr4Request) (*E2EAttributeGetAttributeAttr4Response, error) {
	return &E2EAttributeGetAttributeAttr4Response{Return: s.attr4}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr4(ctx context.Context, req *E2EAttributeSetAttributeAttr4Request) (*E2EAttributeSetAttributeAttr4Response, error) {
	s.attr4 = req.Attr4
	return &E2EAttributeSetAttributeAttr4Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr5(ctx context.Context, req *E2EAttributeGetAttributeAttr5Request) (*E2EAttributeGetAttributeAttr5Response, error) {
	return &E2EAttributeGetAttributeAttr5Response{Return: s.attr5}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr5(ctx context.Context, req *E2EAttributeSetAttributeAttr5Request) (*E2EAttributeSetAttributeAttr5Response, error) {
	s.attr5 = req.Attr5
	return &E2EAttributeSetAttributeAttr5Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr6(ctx context.Context, req *E2EAttributeGetAttributeAttr6Request) (*E2EAttributeGetAttributeAttr6Response, error) {
	return &E2EAttributeGetAttributeAttr6Response{Return: s.attr6}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr6(ctx context.Context, req *E2EAttributeSetAttributeAttr6Request) (*E2EAttributeSetAttributeAttr6Response, error) {
	s.attr6 = req.Attr6
	return &E2EAttributeSetAttributeAttr6Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr61(ctx context.Context, req *E2EAttributeGetAttributeAttr61Request) (*E2EAttributeGetAttributeAttr61Response, error) {
	return &E2EAttributeGetAttributeAttr61Response{Return: s.attr61}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr61(ctx context.Context, req *E2EAttributeSetAttributeAttr61Request) (*E2EAttributeSetAttributeAttr61Response, error) {
	s.attr61 = req.Attr61
	return &E2EAttributeSetAttributeAttr61Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr7(ctx context.Context, req *E2EAttributeGetAttributeAttr7Request) (*E2EAttributeGetAttributeAttr7Response, error) {
	return &E2EAttributeGetAttributeAttr7Response{Return: s.attr7}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr7(ctx context.Context, req *E2EAttributeSetAttributeAttr7Request) (*E2EAttributeSetAttributeAttr7Response, error) {
	s.attr7 = req.Attr7
	return &E2EAttributeSetAttributeAttr7Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr8(ctx context.Context, req *E2EAttributeGetAttributeAttr8Request) (*E2EAttributeGetAttributeAttr8Response, error) {
	return &E2EAttributeGetAttributeAttr8Response{Return: s.attr8}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr8(ctx context.Context, req *E2EAttributeSetAttributeAttr8Request) (*E2EAttributeSetAttributeAttr8Response, error) {
	s.attr8 = req.Attr8
	return &E2EAttributeSetAttributeAttr8Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr9(ctx context.Context, req *E2EAttributeGetAttributeAttr9Request) (*E2EAttributeGetAttributeAttr9Response, error) {
	return &E2EAttributeGetAttributeAttr9Response{Return: s.attr9}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr9(ctx context.Context, req *E2EAttributeSetAttributeAttr9Request) (*E2EAttributeSetAttributeAttr9Response, error) {
	s.attr9 = req.Attr9
	return &E2EAttributeSetAttributeAttr9Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr10(ctx context.Context, req *E2EAttributeGetAttributeAttr10Request) (*E2EAttributeGetAttributeAttr10Response, error) {
	return &E2EAttributeGetAttributeAttr10Response{Return: s.attr10}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr10(ctx context.Context, req *E2EAttributeSetAttributeAttr10Request) (*E2EAttributeSetAttributeAttr10Response, error) {
	s.attr10 = req.Attr10
	return &E2EAttributeSetAttributeAttr10Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr11(ctx context.Context, req *E2EAttributeGetAttributeAttr11Request) (*E2EAttributeGetAttributeAttr11Response, error) {
	return &E2EAttributeGetAttributeAttr11Response{Return: s.attr11}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr11(ctx context.Context, req *E2EAttributeSetAttributeAttr11Request) (*E2EAttributeSetAttributeAttr11Response, error) {
	s.attr11 = req.Attr11
	return &E2EAttributeSetAttributeAttr11Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr12(ctx context.Context, req *E2EAttributeGetAttributeAttr12Request) (*E2EAttributeGetAttributeAttr12Response, error) {
	return &E2EAttributeGetAttributeAttr12Response{Return: s.attr12}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr12(ctx context.Context, req *E2EAttributeSetAttributeAttr12Request) (*E2EAttributeSetAttributeAttr12Response, error) {
	s.attr12 = req.Attr12
	return &E2EAttributeSetAttributeAttr12Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr13(ctx context.Context, req *E2EAttributeGetAttributeAttr13Request) (*E2EAttributeGetAttributeAttr13Response, error) {
	return &E2EAttributeGetAttributeAttr13Response{Return: s.attr13}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr13(ctx context.Context, req *E2EAttributeSetAttributeAttr13Request) (*E2EAttributeSetAttributeAttr13Response, error) {
	s.attr13 = req.Attr13
	return &E2EAttributeSetAttributeAttr13Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr14(ctx context.Context, req *E2EAttributeGetAttributeAttr14Request) (*E2EAttributeGetAttributeAttr14Response, error) {
	return &E2EAttributeGetAttributeAttr14Response{Return: s.attr14}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr14(ctx context.Context, req *E2EAttributeSetAttributeAttr14Request) (*E2EAttributeSetAttributeAttr14Response, error) {
	s.attr14 = req.Attr14
	return &E2EAttributeSetAttributeAttr14Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr15(ctx context.Context, req *E2EAttributeGetAttributeAttr15Request) (*E2EAttributeGetAttributeAttr15Response, error) {
	return &E2EAttributeGetAttributeAttr15Response{Return: s.attr15}, nil
}

func (s *MyE2EAttribute) SetAttributeAttr15(ctx context.Context, req *E2EAttributeSetAttributeAttr15Request) (*E2EAttributeSetAttributeAttr15Response, error) {
	s.attr15 = req.Attr15
	return &E2EAttributeSetAttributeAttr15Response{}, nil
}

func (s *MyE2EAttribute) GetAttributeAttr16(ctx context.Context, req *E2EAttributeGetAttributeAttr16Request) (*E2EAttributeGetAttributeAttr16Response, error) {
	return &E2EAttributeGetAttributeAttr16Response{Return: s.attr16}, nil
}

type MyE2EHttpForm struct{}

func (s *MyE2EHttpForm) SubmitProfile(ctx context.Context, req *E2EHttpFormSubmitProfileRequest) (*E2EHttpFormSubmitProfileResponse, error) {
	return &E2EHttpFormSubmitProfileResponse{
		Return:         fmt.Sprintf("name:%s,age:%s", req.Name, formatOptInt(req.Age)),
		NormalizedName: strings.ToUpper(req.Name),
	}, nil
}

type MyE2EHttpScopeMatrix struct{}

func (s *MyE2EHttpScopeMatrix) GetAttributeScopeInheritedAttr(ctx context.Context, req *E2EHttpScopeMatrixGetAttributeScopeInheritedAttrRequest) (*E2EHttpScopeMatrixGetAttributeScopeInheritedAttrResponse, error) {
	return &E2EHttpScopeMatrixGetAttributeScopeInheritedAttrResponse{Return: "inherited"}, nil
}

func (s *MyE2EHttpScopeMatrix) GetAttributeScopeBareAttr(ctx context.Context, req *E2EHttpScopeMatrixGetAttributeScopeBareAttrRequest) (*E2EHttpScopeMatrixGetAttributeScopeBareAttrResponse, error) {
	return &E2EHttpScopeMatrixGetAttributeScopeBareAttrResponse{Return: "bare"}, nil
}

func (s *MyE2EHttpScopeMatrix) DefaultScope(ctx context.Context, req *E2EHttpScopeMatrixDefaultScopeRequest) (*E2EHttpScopeMatrixDefaultScopeResponse, error) {
	return &E2EHttpScopeMatrixDefaultScopeResponse{Return: req.RequestBody.Name}, nil
}
func (s *MyE2EHttpScopeMatrix) OverrideConsumesOnly(ctx context.Context, req *E2EHttpScopeMatrixOverrideConsumesOnlyRequest) (*E2EHttpScopeMatrixOverrideConsumesOnlyResponse, error) {
	return &E2EHttpScopeMatrixOverrideConsumesOnlyResponse{
		Return:         fmt.Sprintf("name:%s,age:%s", req.Name, formatOptInt(req.Age)),
		NormalizedName: strings.ToUpper(req.Name),
	}, nil
}
func (s *MyE2EHttpScopeMatrix) OverrideProducesOnly(ctx context.Context, req *E2EHttpScopeMatrixOverrideProducesOnlyRequest) (*E2EHttpScopeMatrixOverrideProducesOnlyResponse, error) {
	return &E2EHttpScopeMatrixOverrideProducesOnlyResponse{
		Return:   StructHttpBody{Name: req.ResourceId},
		Revision: 1,
	}, nil
}
func (s *MyE2EHttpScopeMatrix) OverrideBothMedia(ctx context.Context, req *E2EHttpScopeMatrixOverrideBothMediaRequest) (*E2EHttpScopeMatrixOverrideBothMediaResponse, error) {
	return &E2EHttpScopeMatrixOverrideBothMediaResponse{
		Return:          StructHttpBody{Name: req.Name, Tags: []string{fmt.Sprintf("age:%s", formatOptInt(req.Age))}},
		NormalizedName: "OVERRIDDEN",
	}, nil
}
func (s *MyE2EHttpScopeMatrix) DeprecatedPlain(ctx context.Context, req *E2EHttpScopeMatrixDeprecatedPlainRequest) (*E2EHttpScopeMatrixDeprecatedPlainResponse, error) {
	return &E2EHttpScopeMatrixDeprecatedPlainResponse{Return: req.ResourceId}, nil
}
func (s *MyE2EHttpScopeMatrix) DeprecatedSinceOnly(ctx context.Context, req *E2EHttpScopeMatrixDeprecatedSinceOnlyRequest) (*E2EHttpScopeMatrixDeprecatedSinceOnlyResponse, error) {
	return &E2EHttpScopeMatrixDeprecatedSinceOnlyResponse{Return: req.ResourceId}, nil
}
func (s *MyE2EHttpScopeMatrix) DeprecatedWindow(ctx context.Context, req *E2EHttpScopeMatrixDeprecatedWindowRequest) (*E2EHttpScopeMatrixDeprecatedWindowResponse, error) {
	return &E2EHttpScopeMatrixDeprecatedWindowResponse{Return: req.ResourceId}, nil
}

type MyE2EHttpDefaultsMatrix struct{}

func (s *MyE2EHttpDefaultsMatrix) DeleteResourceDefaultQuery(ctx context.Context, req *E2EHttpDefaultsMatrixDeleteResourceDefaultQueryRequest) (*E2EHttpDefaultsMatrixDeleteResourceDefaultQueryResponse, error) {
	return &E2EHttpDefaultsMatrixDeleteResourceDefaultQueryResponse{Return: fmt.Sprintf("%s:%d", req.Id, req.Revision)}, nil
}
func (s *MyE2EHttpDefaultsMatrix) ProbeResourceDefaultQuery(ctx context.Context, req *E2EHttpDefaultsMatrixProbeResourceDefaultQueryRequest) (*E2EHttpDefaultsMatrixProbeResourceDefaultQueryResponse, error) {
	return &E2EHttpDefaultsMatrixProbeResourceDefaultQueryResponse{}, nil
}
func (s *MyE2EHttpDefaultsMatrix) ResourceOptionsDefaultQuery(ctx context.Context, req *E2EHttpDefaultsMatrixResourceOptionsDefaultQueryRequest) (*E2EHttpDefaultsMatrixResourceOptionsDefaultQueryResponse, error) {
	return &E2EHttpDefaultsMatrixResourceOptionsDefaultQueryResponse{}, nil
}
func (s *MyE2EHttpDefaultsMatrix) ReplaceResourceDefaultBody(ctx context.Context, req *E2EHttpDefaultsMatrixReplaceResourceDefaultBodyRequest) (*E2EHttpDefaultsMatrixReplaceResourceDefaultBodyResponse, error) {
	return &E2EHttpDefaultsMatrixReplaceResourceDefaultBodyResponse{Return: StructHttpBody{Name: req.Name, Alias: req.Alias, Tags: []string{req.Id}}}, nil
}
func (s *MyE2EHttpDefaultsMatrix) PatchResourceDefaultBody(ctx context.Context, req *E2EHttpDefaultsMatrixPatchResourceDefaultBodyRequest) (*E2EHttpDefaultsMatrixPatchResourceDefaultBodyResponse, error) {
	return &E2EHttpDefaultsMatrixPatchResourceDefaultBodyResponse{Return: StructHttpBody{Name: req.Name, Alias: req.Alias, Tags: []string{req.Id}}}, nil
}

type MyE2EHttpSecurityMatrix struct{}

func (s *MyE2EHttpSecurityMatrix) InheritedSecurity(ctx context.Context, req *E2EHttpSecurityMatrixInheritedSecurityRequest) (*E2EHttpSecurityMatrixInheritedSecurityResponse, error) {
	return &E2EHttpSecurityMatrixInheritedSecurityResponse{Return: req.ResourceId + ":" + req.TraceId}, nil
}
func (s *MyE2EHttpSecurityMatrix) BearerOrCookieSecurity(ctx context.Context, req *E2EHttpSecurityMatrixBearerOrCookieSecurityRequest) (*E2EHttpSecurityMatrixBearerOrCookieSecurityResponse, error) {
	return &E2EHttpSecurityMatrixBearerOrCookieSecurityResponse{Return: fmt.Sprintf("%s:%s", req.Action, formatOpt(req.Note))}, nil
}
func (s *MyE2EHttpSecurityMatrix) AlternativeSecurity(ctx context.Context, req *E2EHttpSecurityMatrixAlternativeSecurityRequest) (*E2EHttpSecurityMatrixAlternativeSecurityResponse, error) {
	return &E2EHttpSecurityMatrixAlternativeSecurityResponse{Return: fmt.Sprintf("%s:%s", req.ResourceId, formatOpt(req.Locale))}, nil
}
func (s *MyE2EHttpSecurityMatrix) OauthSecurity(ctx context.Context, req *E2EHttpSecurityMatrixOauthSecurityRequest) (*E2EHttpSecurityMatrixOauthSecurityResponse, error) {
	return &E2EHttpSecurityMatrixOauthSecurityResponse{Return: fmt.Sprintf("%s:%s", req.Keyword, formatOptInt(req.Page))}, nil
}
func (s *MyE2EHttpSecurityMatrix) PublicPing(ctx context.Context, req *E2EHttpSecurityMatrixPublicPingRequest) (*E2EHttpSecurityMatrixPublicPingResponse, error) {
	return &E2EHttpSecurityMatrixPublicPingResponse{Return: "pong"}, nil
}

func main() {
	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()
	RegisterE2EPathSeverHandler(r, &MyE2EPathSever{})
	RegisterE2EHttpRouteAndBodyHandler(r, &MyE2EHttpRouteAndBody{})
	RegisterE2EHttpSecurityHandler(r, &MyE2EHttpSecurity{})

	typeServer := &MyE2ETypeServer{attr1: "attr1", attr2: []string{"attr2"}}
	RegisterE2ETypeServerHandler(r, typeServer)

	attr := &MyE2EAttribute{
		attr1:  "attr1",
		attr2:  []string{"attr2"},
		attr4:  EnumSimple1V1,
		attr5:  StructEmpty{},
		attr61: UnionSimple{Tag: "V1", Value: map[string]any{"data": 1}},
	}
	RegisterE2EAttributeHandler(r, attr)

	RegisterE2EHttpFormHandler(r, &MyE2EHttpForm{})
	RegisterE2EHttpScopeMatrixHandler(r, &MyE2EHttpScopeMatrix{})
	RegisterE2EHttpDefaultsMatrixHandler(r, &MyE2EHttpDefaultsMatrix{})
	RegisterE2EHttpSecurityMatrixHandler(r, &MyE2EHttpSecurityMatrix{})

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	fmt.Printf("Go server starting on port %s\n", port)
	http.ListenAndServe(fmt.Sprintf(":%s", port), r)
}
