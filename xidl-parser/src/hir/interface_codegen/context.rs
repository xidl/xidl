use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct TemplateContext {
    pub modules: Vec<String>,
    pub interface_name: String,
    pub consts: Vec<ConstContext>,
    pub operations: Vec<OperationContext>,
}

#[derive(Serialize, Clone)]
pub struct ConstContext {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Clone)]
pub struct OperationContext {
    pub name: String,
    pub in_members: Vec<MemberContext>,
    pub out_members: Vec<MemberContext>,
    pub return_ty: Option<String>,
    pub result_exceptions: Vec<ExceptionContext>,
}

#[derive(Clone, Serialize)]
pub struct MemberContext {
    pub ty: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct ExceptionContext {
    pub const_name: String,
    pub member_name: String,
    pub ty: String,
}
