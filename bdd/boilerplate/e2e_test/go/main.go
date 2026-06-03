package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

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

type MyE2ePathSever struct{}

func (s *MyE2ePathSever) OpWithPath(ctx context.Context, req *E2ePathSeverOpWithPathRequest) (*E2ePathSeverOpWithPathResponse, error) {
	return &E2ePathSeverOpWithPathResponse{Return: []string{req.Param1}}, nil
}

func (s *MyE2ePathSever) OpWithQuery(ctx context.Context, req *E2ePathSeverOpWithQueryRequest) (*E2ePathSeverOpWithQueryResponse, error) {
	return &E2ePathSeverOpWithQueryResponse{Return: []string{req.Param1, req.Q}}, nil
}

func (s *MyE2ePathSever) OpWithParams(ctx context.Context, req *E2ePathSeverOpWithParamsRequest) (*E2ePathSeverOpWithParamsResponse, error) {
	// Format body and a map to match the test assertion
	res := []string{req.PathName}
	res = append(res, req.Q...)
	res = append(res, fmt.Sprintf("%v", req.B))
	res = append(res, fmt.Sprintf("%v", req.A))
	return &E2ePathSeverOpWithParamsResponse{Return: res}, nil
}

func (s *MyE2ePathSever) OpWithQuery2(ctx context.Context, req *E2ePathSeverOpWithQuery2Request) (*E2ePathSeverOpWithQuery2Response, error) {
	return &E2ePathSeverOpWithQuery2Response{Return: req.All + ":" + req.Word + ":" + req.Q}, nil
}

type MyE2eHttpRouteAndBody struct{}

func (s *MyE2eHttpRouteAndBody) GetResource(ctx context.Context, req *E2eHttpRouteAndBodyGetResourceRequest) (*E2eHttpRouteAndBodyGetResourceResponse, error) {
	return &E2eHttpRouteAndBodyGetResourceResponse{Return: fmt.Sprintf("id:%s,lang:%s,trace:%s", req.ResourceId, formatOpt(req.Locale), req.XTraceId)}, nil
}

func (s *MyE2eHttpRouteAndBody) GetFile(ctx context.Context, req *E2eHttpRouteAndBodyGetFileRequest) (*E2eHttpRouteAndBodyGetFileResponse, error) {
	return &E2eHttpRouteAndBodyGetFileResponse{Return: fmt.Sprintf("file:%s,download:%t,version:%s", req.FilePath, req.Download, formatOpt(req.Version))}, nil
}

func (s *MyE2eHttpRouteAndBody) CreateResource(ctx context.Context, req *E2eHttpRouteAndBodyCreateResourceRequest) (*E2eHttpRouteAndBodyCreateResourceResponse, error) {
	return &E2eHttpRouteAndBodyCreateResourceResponse{Return: req.ResourceBody}, nil
}

func (s *MyE2eHttpRouteAndBody) ReplaceResource(ctx context.Context, req *E2eHttpRouteAndBodyReplaceResourceRequest) (*E2eHttpRouteAndBodyReplaceResourceResponse, error) {
	return &E2eHttpRouteAndBodyReplaceResourceResponse{}, nil
}

func (s *MyE2eHttpRouteAndBody) PatchResource(ctx context.Context, req *E2eHttpRouteAndBodyPatchResourceRequest) (*E2eHttpRouteAndBodyPatchResourceResponse, error) {
	return &E2eHttpRouteAndBodyPatchResourceResponse{Return: req.Changes}, nil
}

func (s *MyE2eHttpRouteAndBody) DeleteResource(ctx context.Context, req *E2eHttpRouteAndBodyDeleteResourceRequest) (*E2eHttpRouteAndBodyDeleteResourceResponse, error) {
	return &E2eHttpRouteAndBodyDeleteResourceResponse{}, nil
}

func (s *MyE2eHttpRouteAndBody) ProbeResource(ctx context.Context, req *E2eHttpRouteAndBodyProbeResourceRequest) (*E2eHttpRouteAndBodyProbeResourceResponse, error) {
	return &E2eHttpRouteAndBodyProbeResourceResponse{}, nil
}

func (s *MyE2eHttpRouteAndBody) ResourceOptions(ctx context.Context, req *E2eHttpRouteAndBodyResourceOptionsRequest) (*E2eHttpRouteAndBodyResourceOptionsResponse, error) {
	return &E2eHttpRouteAndBodyResourceOptionsResponse{}, nil
}

func (s *MyE2eHttpRouteAndBody) GetMsgpackResource(ctx context.Context, req *E2eHttpRouteAndBodyGetMsgpackResourceRequest) (*E2eHttpRouteAndBodyGetMsgpackResourceResponse, error) {
	return &E2eHttpRouteAndBodyGetMsgpackResourceResponse{Return: StructHttpBody{Name: "msgpack"}, Revision: 1}, nil
}

func (s *MyE2eHttpRouteAndBody) DedupResource(ctx context.Context, req *E2eHttpRouteAndBodyDedupResourceRequest) (*E2eHttpRouteAndBodyDedupResourceResponse, error) {
	return &E2eHttpRouteAndBodyDedupResourceResponse{Return: req.Id + ":" + req.XTraceId}, nil
}

func (s *MyE2eHttpRouteAndBody) PreviewResource(ctx context.Context, req *E2eHttpRouteAndBodyPreviewResourceRequest) (*E2eHttpRouteAndBodyPreviewResourceResponse, error) {
	return &E2eHttpRouteAndBodyPreviewResourceResponse{Return: req.Resource}, nil
}

type MyE2eHttpSecurity struct{}

func (s *MyE2eHttpSecurity) GetSecureUser(ctx context.Context, req *E2eHttpSecurityGetSecureUserRequest) (*E2eHttpSecurityGetSecureUserResponse, error) {
	return &E2eHttpSecurityGetSecureUserResponse{Return: fmt.Sprintf("user:%s,lang:%s,trace:%s", req.UserId, formatOpt(req.Locale), req.XTraceId)}, nil
}

func (s *MyE2eHttpSecurity) SearchSecureUser(ctx context.Context, req *E2eHttpSecuritySearchSecureUserRequest) (*E2eHttpSecuritySearchSecureUserResponse, error) {
	return &E2eHttpSecuritySearchSecureUserResponse{Return: fmt.Sprintf("keyword:%s,page:%s", req.Keyword, formatOptInt(req.Page))}, nil
}

func (s *MyE2eHttpSecurity) Healthz(ctx context.Context, req *E2eHttpSecurityHealthzRequest) (*E2eHttpSecurityHealthzResponse, error) {
	return &E2eHttpSecurityHealthzResponse{Return: "ok"}, nil
}

type MyE2eTypeServer struct {
	attr1 string
	attr2 []string
}

func (s *MyE2eTypeServer) GetAttributeTypeAttr1(ctx context.Context, req *E2eTypeServerGetAttributeTypeAttr1Request) (*E2eTypeServerGetAttributeTypeAttr1Response, error) {
	return &E2eTypeServerGetAttributeTypeAttr1Response{Return: s.attr1}, nil
}

func (s *MyE2eTypeServer) SetAttributeTypeAttr1(ctx context.Context, req *E2eTypeServerSetAttributeTypeAttr1Request) (*E2eTypeServerSetAttributeTypeAttr1Response, error) {
	s.attr1 = req.Value
	return &E2eTypeServerSetAttributeTypeAttr1Response{}, nil
}

func (s *MyE2eTypeServer) GetAttributeTypeAttr2(ctx context.Context, req *E2eTypeServerGetAttributeTypeAttr2Request) (*E2eTypeServerGetAttributeTypeAttr2Response, error) {
	return &E2eTypeServerGetAttributeTypeAttr2Response{Return: s.attr2}, nil
}

func (s *MyE2eTypeServer) SimpleOp(ctx context.Context, req *E2eTypeServerSimpleOpRequest) (*E2eTypeServerSimpleOpResponse, error) {
	return &E2eTypeServerSimpleOpResponse{}, nil
}

func (s *MyE2eTypeServer) SimpleOpWithReturn1(ctx context.Context, req *E2eTypeServerSimpleOpWithReturn1Request) (*E2eTypeServerSimpleOpWithReturn1Response, error) {
	return &E2eTypeServerSimpleOpWithReturn1Response{Return: "simple_op_with_return1"}, nil
}

func (s *MyE2eTypeServer) SimpleOpWithReturn2(ctx context.Context, req *E2eTypeServerSimpleOpWithReturn2Request) (*E2eTypeServerSimpleOpWithReturn2Response, error) {
	return &E2eTypeServerSimpleOpWithReturn2Response{}, nil
}

func (s *MyE2eTypeServer) SimpleOpWithReturn3(ctx context.Context, req *E2eTypeServerSimpleOpWithReturn3Request) (*E2eTypeServerSimpleOpWithReturn3Response, error) {
	return &E2eTypeServerSimpleOpWithReturn3Response{Return: EnumSimple1V1}, nil
}

func (s *MyE2eTypeServer) SimpleOpWithReturn4(ctx context.Context, req *E2eTypeServerSimpleOpWithReturn4Request) (*E2eTypeServerSimpleOpWithReturn4Response, error) {
	return &E2eTypeServerSimpleOpWithReturn4Response{Return: StructEmpty{}}, nil
}

func (s *MyE2eTypeServer) SimpleOpWithReturn5(ctx context.Context, req *E2eTypeServerSimpleOpWithReturn5Request) (*E2eTypeServerSimpleOpWithReturn5Response, error) {
	return &E2eTypeServerSimpleOpWithReturn5Response{}, nil
}

func (s *MyE2eTypeServer) ReturnWithSequence1(ctx context.Context, req *E2eTypeServerReturnWithSequence1Request) (*E2eTypeServerReturnWithSequence1Response, error) {
	return &E2eTypeServerReturnWithSequence1Response{Return: []string{"s1", "s2"}}, nil
}

func (s *MyE2eTypeServer) ReturnWithSequence2(ctx context.Context, req *E2eTypeServerReturnWithSequence2Request) (*E2eTypeServerReturnWithSequence2Response, error) {
	return &E2eTypeServerReturnWithSequence2Response{}, nil
}

func (s *MyE2eTypeServer) ReturnWithSequence3(ctx context.Context, req *E2eTypeServerReturnWithSequence3Request) (*E2eTypeServerReturnWithSequence3Response, error) {
	return &E2eTypeServerReturnWithSequence3Response{Return: []EnumSimple1{EnumSimple1V1, EnumSimple1V2}}, nil
}

func (s *MyE2eTypeServer) ReturnWithSequence4(ctx context.Context, req *E2eTypeServerReturnWithSequence4Request) (*E2eTypeServerReturnWithSequence4Response, error) {
	return &E2eTypeServerReturnWithSequence4Response{Return: []StructEmpty{{}}}, nil
}

func (s *MyE2eTypeServer) ReturnWithSequence5(ctx context.Context, req *E2eTypeServerReturnWithSequence5Request) (*E2eTypeServerReturnWithSequence5Response, error) {
	return &E2eTypeServerReturnWithSequence5Response{}, nil
}

func (s *MyE2eTypeServer) ReturnWithMap(ctx context.Context, req *E2eTypeServerReturnWithMapRequest) (*E2eTypeServerReturnWithMapResponse, error) {
	return &E2eTypeServerReturnWithMapResponse{Return: map[string]uint8{"k1": 1}}, nil
}

func (s *MyE2eTypeServer) ReturnWithAny(ctx context.Context, req *E2eTypeServerReturnWithAnyRequest) (*E2eTypeServerReturnWithAnyResponse, error) {
	return &E2eTypeServerReturnWithAnyResponse{Return: map[string]any{"any": "value"}}, nil
}

func (s *MyE2eTypeServer) ReturnWithAnySequence(ctx context.Context, req *E2eTypeServerReturnWithAnySequenceRequest) (*E2eTypeServerReturnWithAnySequenceResponse, error) {
	return &E2eTypeServerReturnWithAnySequenceResponse{Return: []any{1, "two"}}, nil
}

func (s *MyE2eTypeServer) ReturnWithAnyMap(ctx context.Context, req *E2eTypeServerReturnWithAnyMapRequest) (*E2eTypeServerReturnWithAnyMapResponse, error) {
	return &E2eTypeServerReturnWithAnyMapResponse{Return: map[string]any{"k1": 1}}, nil
}

func (s *MyE2eTypeServer) ParameterOp(ctx context.Context, req *E2eTypeServerParameterOpRequest) (*E2eTypeServerParameterOpResponse, error) {
	return &E2eTypeServerParameterOpResponse{}, nil
}

func (s *MyE2eTypeServer) ParameterOp2(ctx context.Context, req *E2eTypeServerParameterOp2Request) (*E2eTypeServerParameterOp2Response, error) {
	return &E2eTypeServerParameterOp2Response{}, nil
}

func (s *MyE2eTypeServer) ParameterOp3(ctx context.Context, req *E2eTypeServerParameterOp3Request) (*E2eTypeServerParameterOp3Response, error) {
	return &E2eTypeServerParameterOp3Response{B: 3, C: []any{}}, nil
}

func (s *MyE2eTypeServer) ParameterOp4(ctx context.Context, req *E2eTypeServerParameterOp4Request) (*E2eTypeServerParameterOp4Response, error) {
	return &E2eTypeServerParameterOp4Response{A: "op4", B: 4, C: []any{}}, nil
}

func (s *MyE2eTypeServer) ParameterOp5(ctx context.Context, req *E2eTypeServerParameterOp5Request) (*E2eTypeServerParameterOp5Response, error) {
	return &E2eTypeServerParameterOp5Response{Return: []any{"op5"}, A: "op5", B: 5, C: []any{}}, nil
}

func (s *MyE2eTypeServer) ParameterOp6(ctx context.Context, req *E2eTypeServerParameterOp6Request) (*E2eTypeServerParameterOp6Response, error) {
	return &E2eTypeServerParameterOp6Response{Return: map[string]any{}, A: "op6", B: 6, C: []any{}}, nil
}

type MyE2eAttribute struct {
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
}

func (s *MyE2eAttribute) GetAttributeAttr1(ctx context.Context, req *E2eAttributeGetAttributeAttr1Request) (*E2eAttributeGetAttributeAttr1Response, error) {
	return &E2eAttributeGetAttributeAttr1Response{Return: s.attr1}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr1(ctx context.Context, req *E2eAttributeSetAttributeAttr1Request) (*E2eAttributeSetAttributeAttr1Response, error) {
	s.attr1 = req.Value
	return &E2eAttributeSetAttributeAttr1Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr2(ctx context.Context, req *E2eAttributeGetAttributeAttr2Request) (*E2eAttributeGetAttributeAttr2Response, error) {
	return &E2eAttributeGetAttributeAttr2Response{Return: s.attr2}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr3(ctx context.Context, req *E2eAttributeGetAttributeAttr3Request) (*E2eAttributeGetAttributeAttr3Response, error) {
	return &E2eAttributeGetAttributeAttr3Response{Return: s.attr3}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr3(ctx context.Context, req *E2eAttributeSetAttributeAttr3Request) (*E2eAttributeSetAttributeAttr3Response, error) {
	s.attr3 = req.Value
	return &E2eAttributeSetAttributeAttr3Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr4(ctx context.Context, req *E2eAttributeGetAttributeAttr4Request) (*E2eAttributeGetAttributeAttr4Response, error) {
	return &E2eAttributeGetAttributeAttr4Response{Return: s.attr4}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr4(ctx context.Context, req *E2eAttributeSetAttributeAttr4Request) (*E2eAttributeSetAttributeAttr4Response, error) {
	s.attr4 = req.Value
	return &E2eAttributeSetAttributeAttr4Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr5(ctx context.Context, req *E2eAttributeGetAttributeAttr5Request) (*E2eAttributeGetAttributeAttr5Response, error) {
	return &E2eAttributeGetAttributeAttr5Response{Return: s.attr5}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr5(ctx context.Context, req *E2eAttributeSetAttributeAttr5Request) (*E2eAttributeSetAttributeAttr5Response, error) {
	s.attr5 = req.Value
	return &E2eAttributeSetAttributeAttr5Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr6(ctx context.Context, req *E2eAttributeGetAttributeAttr6Request) (*E2eAttributeGetAttributeAttr6Response, error) {
	return &E2eAttributeGetAttributeAttr6Response{Return: s.attr6}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr6(ctx context.Context, req *E2eAttributeSetAttributeAttr6Request) (*E2eAttributeSetAttributeAttr6Response, error) {
	s.attr6 = req.Value
	return &E2eAttributeSetAttributeAttr6Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr61(ctx context.Context, req *E2eAttributeGetAttributeAttr61Request) (*E2eAttributeGetAttributeAttr61Response, error) {
	return &E2eAttributeGetAttributeAttr61Response{Return: s.attr61}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr61(ctx context.Context, req *E2eAttributeSetAttributeAttr61Request) (*E2eAttributeSetAttributeAttr61Response, error) {
	s.attr61 = req.Value
	return &E2eAttributeSetAttributeAttr61Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr7(ctx context.Context, req *E2eAttributeGetAttributeAttr7Request) (*E2eAttributeGetAttributeAttr7Response, error) {
	return &E2eAttributeGetAttributeAttr7Response{Return: s.attr7}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr7(ctx context.Context, req *E2eAttributeSetAttributeAttr7Request) (*E2eAttributeSetAttributeAttr7Response, error) {
	s.attr7 = req.Value
	return &E2eAttributeSetAttributeAttr7Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr8(ctx context.Context, req *E2eAttributeGetAttributeAttr8Request) (*E2eAttributeGetAttributeAttr8Response, error) {
	return &E2eAttributeGetAttributeAttr8Response{Return: s.attr8}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr8(ctx context.Context, req *E2eAttributeSetAttributeAttr8Request) (*E2eAttributeSetAttributeAttr8Response, error) {
	s.attr8 = req.Value
	return &E2eAttributeSetAttributeAttr8Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr9(ctx context.Context, req *E2eAttributeGetAttributeAttr9Request) (*E2eAttributeGetAttributeAttr9Response, error) {
	return &E2eAttributeGetAttributeAttr9Response{Return: s.attr9}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr9(ctx context.Context, req *E2eAttributeSetAttributeAttr9Request) (*E2eAttributeSetAttributeAttr9Response, error) {
	s.attr9 = req.Value
	return &E2eAttributeSetAttributeAttr9Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr10(ctx context.Context, req *E2eAttributeGetAttributeAttr10Request) (*E2eAttributeGetAttributeAttr10Response, error) {
	return &E2eAttributeGetAttributeAttr10Response{Return: s.attr10}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr10(ctx context.Context, req *E2eAttributeSetAttributeAttr10Request) (*E2eAttributeSetAttributeAttr10Response, error) {
	s.attr10 = req.Value
	return &E2eAttributeSetAttributeAttr10Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr11(ctx context.Context, req *E2eAttributeGetAttributeAttr11Request) (*E2eAttributeGetAttributeAttr11Response, error) {
	return &E2eAttributeGetAttributeAttr11Response{Return: s.attr11}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr11(ctx context.Context, req *E2eAttributeSetAttributeAttr11Request) (*E2eAttributeSetAttributeAttr11Response, error) {
	s.attr11 = req.Value
	return &E2eAttributeSetAttributeAttr11Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr12(ctx context.Context, req *E2eAttributeGetAttributeAttr12Request) (*E2eAttributeGetAttributeAttr12Response, error) {
	return &E2eAttributeGetAttributeAttr12Response{Return: s.attr12}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr12(ctx context.Context, req *E2eAttributeSetAttributeAttr12Request) (*E2eAttributeSetAttributeAttr12Response, error) {
	s.attr12 = req.Value
	return &E2eAttributeSetAttributeAttr12Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr13(ctx context.Context, req *E2eAttributeGetAttributeAttr13Request) (*E2eAttributeGetAttributeAttr13Response, error) {
	return &E2eAttributeGetAttributeAttr13Response{Return: s.attr13}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr13(ctx context.Context, req *E2eAttributeSetAttributeAttr13Request) (*E2eAttributeSetAttributeAttr13Response, error) {
	s.attr13 = req.Value
	return &E2eAttributeSetAttributeAttr13Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr14(ctx context.Context, req *E2eAttributeGetAttributeAttr14Request) (*E2eAttributeGetAttributeAttr14Response, error) {
	return &E2eAttributeGetAttributeAttr14Response{Return: s.attr14}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr14(ctx context.Context, req *E2eAttributeSetAttributeAttr14Request) (*E2eAttributeSetAttributeAttr14Response, error) {
	s.attr14 = req.Value
	return &E2eAttributeSetAttributeAttr14Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr15(ctx context.Context, req *E2eAttributeGetAttributeAttr15Request) (*E2eAttributeGetAttributeAttr15Response, error) {
	return &E2eAttributeGetAttributeAttr15Response{Return: s.attr15}, nil
}
func (s *MyE2eAttribute) SetAttributeAttr15(ctx context.Context, req *E2eAttributeSetAttributeAttr15Request) (*E2eAttributeSetAttributeAttr15Response, error) {
	s.attr15 = req.Value
	return &E2eAttributeSetAttributeAttr15Response{}, nil
}
func (s *MyE2eAttribute) GetAttributeAttr16(ctx context.Context, req *E2eAttributeGetAttributeAttr16Request) (*E2eAttributeGetAttributeAttr16Response, error) {
	return &E2eAttributeGetAttributeAttr16Response{Return: "attr16"}, nil
}

type MyE2eHttpForm struct{}

func (s *MyE2eHttpForm) SubmitProfile(ctx context.Context, req *E2eHttpFormSubmitProfileRequest) (*E2eHttpFormSubmitProfileResponse, error) {
	return &E2eHttpFormSubmitProfileResponse{
		Return:         fmt.Sprintf("name:%s,age:%s", req.Name, formatOptInt(req.Age)),
		NormalizedName: strings.ToUpper(req.Name),
	}, nil
}

type MyE2eHttpScopeMatrix struct{}

func (s *MyE2eHttpScopeMatrix) GetAttributeScopeInheritedAttr(ctx context.Context, req *E2eHttpScopeMatrixGetAttributeScopeInheritedAttrRequest) (*E2eHttpScopeMatrixGetAttributeScopeInheritedAttrResponse, error) {
	return &E2eHttpScopeMatrixGetAttributeScopeInheritedAttrResponse{Return: "inherited"}, nil
}
func (s *MyE2eHttpScopeMatrix) GetAttributeScopeBareAttr(ctx context.Context, req *E2eHttpScopeMatrixGetAttributeScopeBareAttrRequest) (*E2eHttpScopeMatrixGetAttributeScopeBareAttrResponse, error) {
	return &E2eHttpScopeMatrixGetAttributeScopeBareAttrResponse{Return: "bare"}, nil
}
func (s *MyE2eHttpScopeMatrix) DefaultScope(ctx context.Context, req *E2eHttpScopeMatrixDefaultScopeRequest) (*E2eHttpScopeMatrixDefaultScopeResponse, error) {
	return &E2eHttpScopeMatrixDefaultScopeResponse{Return: req.RequestBody.Name}, nil
}
func (s *MyE2eHttpScopeMatrix) OverrideConsumesOnly(ctx context.Context, req *E2eHttpScopeMatrixOverrideConsumesOnlyRequest) (*E2eHttpScopeMatrixOverrideConsumesOnlyResponse, error) {
	return &E2eHttpScopeMatrixOverrideConsumesOnlyResponse{
		Return:         fmt.Sprintf("name:%s,age:%s", req.Name, formatOptInt(req.Age)),
		NormalizedName: strings.ToUpper(req.Name),
	}, nil
}
func (s *MyE2eHttpScopeMatrix) OverrideProducesOnly(ctx context.Context, req *E2eHttpScopeMatrixOverrideProducesOnlyRequest) (*E2eHttpScopeMatrixOverrideProducesOnlyResponse, error) {
	return &E2eHttpScopeMatrixOverrideProducesOnlyResponse{
		Return:   StructHttpBody{Name: req.ResourceId},
		Revision: 1,
	}, nil
}
func (s *MyE2eHttpScopeMatrix) OverrideBothMedia(ctx context.Context, req *E2eHttpScopeMatrixOverrideBothMediaRequest) (*E2eHttpScopeMatrixOverrideBothMediaResponse, error) {
	return &E2eHttpScopeMatrixOverrideBothMediaResponse{
		Return:          StructHttpBody{Name: req.Name, Tags: []string{fmt.Sprintf("age:%s", formatOptInt(req.Age))}},
		NormalizedName: "OVERRIDDEN",
	}, nil
}
func (s *MyE2eHttpScopeMatrix) DeprecatedPlain(ctx context.Context, req *E2eHttpScopeMatrixDeprecatedPlainRequest) (*E2eHttpScopeMatrixDeprecatedPlainResponse, error) {
	return &E2eHttpScopeMatrixDeprecatedPlainResponse{Return: req.ResourceId}, nil
}
func (s *MyE2eHttpScopeMatrix) DeprecatedSinceOnly(ctx context.Context, req *E2eHttpScopeMatrixDeprecatedSinceOnlyRequest) (*E2eHttpScopeMatrixDeprecatedSinceOnlyResponse, error) {
	return &E2eHttpScopeMatrixDeprecatedSinceOnlyResponse{Return: req.ResourceId}, nil
}
func (s *MyE2eHttpScopeMatrix) DeprecatedWindow(ctx context.Context, req *E2eHttpScopeMatrixDeprecatedWindowRequest) (*E2eHttpScopeMatrixDeprecatedWindowResponse, error) {
	return &E2eHttpScopeMatrixDeprecatedWindowResponse{Return: req.ResourceId}, nil
}

type MyE2eHttpDefaultsMatrix struct{}

func (s *MyE2eHttpDefaultsMatrix) DeleteResourceDefaultQuery(ctx context.Context, req *E2eHttpDefaultsMatrixDeleteResourceDefaultQueryRequest) (*E2eHttpDefaultsMatrixDeleteResourceDefaultQueryResponse, error) {
	return &E2eHttpDefaultsMatrixDeleteResourceDefaultQueryResponse{Return: fmt.Sprintf("%s:%d", req.Id, req.Revision)}, nil
}
func (s *MyE2eHttpDefaultsMatrix) ProbeResourceDefaultQuery(ctx context.Context, req *E2eHttpDefaultsMatrixProbeResourceDefaultQueryRequest) (*E2eHttpDefaultsMatrixProbeResourceDefaultQueryResponse, error) {
	return &E2eHttpDefaultsMatrixProbeResourceDefaultQueryResponse{}, nil
}
func (s *MyE2eHttpDefaultsMatrix) ResourceOptionsDefaultQuery(ctx context.Context, req *E2eHttpDefaultsMatrixResourceOptionsDefaultQueryRequest) (*E2eHttpDefaultsMatrixResourceOptionsDefaultQueryResponse, error) {
	return &E2eHttpDefaultsMatrixResourceOptionsDefaultQueryResponse{}, nil
}
func (s *MyE2eHttpDefaultsMatrix) ReplaceResourceDefaultBody(ctx context.Context, req *E2eHttpDefaultsMatrixReplaceResourceDefaultBodyRequest) (*E2eHttpDefaultsMatrixReplaceResourceDefaultBodyResponse, error) {
	return &E2eHttpDefaultsMatrixReplaceResourceDefaultBodyResponse{Return: StructHttpBody{Name: req.Name, Alias: req.Alias, Tags: []string{req.Id}}}, nil
}
func (s *MyE2eHttpDefaultsMatrix) PatchResourceDefaultBody(ctx context.Context, req *E2eHttpDefaultsMatrixPatchResourceDefaultBodyRequest) (*E2eHttpDefaultsMatrixPatchResourceDefaultBodyResponse, error) {
	return &E2eHttpDefaultsMatrixPatchResourceDefaultBodyResponse{Return: StructHttpBody{Name: req.Name, Alias: req.Alias, Tags: []string{req.Id}}}, nil
}

type MyE2eHttpSecurityMatrix struct{}

func (s *MyE2eHttpSecurityMatrix) InheritedSecurity(ctx context.Context, req *E2eHttpSecurityMatrixInheritedSecurityRequest) (*E2eHttpSecurityMatrixInheritedSecurityResponse, error) {
	return &E2eHttpSecurityMatrixInheritedSecurityResponse{Return: req.ResourceId + ":" + req.XTraceId}, nil
}
func (s *MyE2eHttpSecurityMatrix) BearerOrCookieSecurity(ctx context.Context, req *E2eHttpSecurityMatrixBearerOrCookieSecurityRequest) (*E2eHttpSecurityMatrixBearerOrCookieSecurityResponse, error) {
	return &E2eHttpSecurityMatrixBearerOrCookieSecurityResponse{Return: fmt.Sprintf("%s:%s", req.Action, formatOpt(req.Note))}, nil
}
func (s *MyE2eHttpSecurityMatrix) AlternativeSecurity(ctx context.Context, req *E2eHttpSecurityMatrixAlternativeSecurityRequest) (*E2eHttpSecurityMatrixAlternativeSecurityResponse, error) {
	return &E2eHttpSecurityMatrixAlternativeSecurityResponse{Return: fmt.Sprintf("%s:%s", req.ResourceId, formatOpt(req.Locale))}, nil
}
func (s *MyE2eHttpSecurityMatrix) OauthSecurity(ctx context.Context, req *E2eHttpSecurityMatrixOauthSecurityRequest) (*E2eHttpSecurityMatrixOauthSecurityResponse, error) {
	return &E2eHttpSecurityMatrixOauthSecurityResponse{Return: fmt.Sprintf("%s:%s", req.Keyword, formatOptInt(req.Page))}, nil
}
func (s *MyE2eHttpSecurityMatrix) PublicPing(ctx context.Context, req *E2eHttpSecurityMatrixPublicPingRequest) (*E2eHttpSecurityMatrixPublicPingResponse, error) {
	return &E2eHttpSecurityMatrixPublicPingResponse{Return: "pong"}, nil
}

func main() {
	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()
	RegisterE2ePathSeverHandler(r, &MyE2ePathSever{})
	RegisterE2eHttpRouteAndBodyHandler(r, &MyE2eHttpRouteAndBody{})
	RegisterE2eHttpSecurityHandler(r, &MyE2eHttpSecurity{})

	typeServer := &MyE2eTypeServer{attr1: "attr1", attr2: []string{"attr2"}}
	RegisterE2eTypeServerHandler(r, typeServer)

	attr := &MyE2eAttribute{attr1: "attr1", attr2: []string{"attr2"}, attr4: EnumSimple1V1, attr5: StructEmpty{}}
	RegisterE2eAttributeHandler(r, attr)

	RegisterE2eHttpFormHandler(r, &MyE2eHttpForm{})
	RegisterE2eHttpScopeMatrixHandler(r, &MyE2eHttpScopeMatrix{})
	RegisterE2eHttpDefaultsMatrixHandler(r, &MyE2eHttpDefaultsMatrix{})
	RegisterE2eHttpSecurityMatrixHandler(r, &MyE2eHttpSecurityMatrix{})

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	fmt.Printf("Go server starting on port %s\n", port)
	http.ListenAndServe(fmt.Sprintf(":%s", port), r)
}
