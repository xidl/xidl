use super::attrs::operations_from_attr;
use super::context::{ExceptionContext, MemberContext, OperationContext};
use super::hash::exception_hash_const_name;
use super::types::{idl_type_spec, scoped_name_to_idl};
use super::*;
use std::collections::HashSet;

#[derive(Clone)]
pub struct OperationInfo {
    pub name: String,
    pub return_ty: ReturnType,
    pub params: Vec<ParamInfo>,
    pub raises: Vec<ScopedName>,
}

#[derive(Clone)]
pub enum ReturnType {
    Void,
    Type(TypeSpec),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ParamMode {
    In,
    Out,
    InOut,
}

#[derive(Clone)]
pub struct ParamInfo {
    pub name: String,
    pub mode: ParamMode,
    pub ty: TypeSpec,
}

pub fn collect_operations(body: &InterfaceBody) -> Vec<OperationInfo> {
    let mut ops = Vec::new();
    let mut existing = HashSet::new();

    for export in &body.0 {
        if let Export::OpDcl(op) = export {
            existing.insert(op.ident.clone());
        }
    }

    for export in &body.0 {
        match export {
            Export::OpDcl(op) => ops.push(operation_from_opdcl(op)),
            Export::AttrDcl(attr) => ops.extend(operations_from_attr(attr, &existing)),
            _ => {}
        }
    }

    ops
}

impl OperationInfo {
    pub fn to_context(&self) -> OperationContext {
        let mut in_members = Vec::new();
        let mut out_members = Vec::new();

        for param in &self.params {
            let member = MemberContext {
                ty: idl_type_spec(&param.ty),
                name: param.name.clone(),
            };
            match param.mode {
                ParamMode::In => in_members.push(member),
                ParamMode::Out => out_members.push(member),
                ParamMode::InOut => {
                    in_members.push(member.clone());
                    out_members.push(member);
                }
            }
        }

        if in_members.is_empty() {
            in_members.push(MemberContext {
                ty: "dds::rpc::UnusedMember".to_string(),
                name: "dummy".to_string(),
            });
        }

        let return_ty = match &self.return_ty {
            ReturnType::Void => None,
            ReturnType::Type(ty) => Some(idl_type_spec(ty)),
        };

        if let Some(return_ty) = &return_ty {
            let return_name = return_member_name(&out_members);
            out_members.push(MemberContext {
                ty: return_ty.clone(),
                name: return_name,
            });
        }

        if return_ty.is_none() && out_members.is_empty() {
            out_members.push(MemberContext {
                ty: "dds::rpc::UnusedMember".to_string(),
                name: "dummy".to_string(),
            });
        }

        let result_exceptions = self
            .raises
            .iter()
            .map(|exception| ExceptionContext {
                const_name: exception_hash_const_name(exception),
                member_name: format!(
                    "{}_ex",
                    exception_unqualified(exception).to_ascii_lowercase()
                ),
                ty: scoped_name_to_idl(exception),
            })
            .collect();

        OperationContext {
            name: self.name.clone(),
            in_members,
            out_members,
            return_ty,
            result_exceptions,
        }
    }
}

fn return_member_name(members: &[MemberContext]) -> String {
    let mut names = HashSet::new();
    for member in members {
        names.insert(member.name.as_str());
    }

    if !names.contains("return_") {
        return "return_".to_string();
    }

    let mut idx = 1;
    loop {
        let candidate = format!("return_{idx}");
        if !names.contains(candidate.as_str()) {
            return candidate;
        }
        idx += 1;
    }
}

fn operation_from_opdcl(op: &OpDcl) -> OperationInfo {
    let return_ty = match &op.ty {
        OpTypeSpec::Void => ReturnType::Void,
        OpTypeSpec::TypeSpec(ty) => ReturnType::Type(ty.clone()),
    };

    let params = op
        .parameter
        .as_ref()
        .map(|params| params.0.iter().map(param_from_dcl).collect())
        .unwrap_or_default();

    let raises = op
        .raises
        .as_ref()
        .map(|raises| raises.0.clone())
        .unwrap_or_default();

    OperationInfo {
        name: op.ident.clone(),
        return_ty,
        params,
        raises,
    }
}

fn param_from_dcl(param: &ParamDcl) -> ParamInfo {
    let mode = match param.attr.as_ref().map(|attr| attr.0.as_str()) {
        Some("out") => ParamMode::Out,
        Some("inout") => ParamMode::InOut,
        _ => ParamMode::In,
    };

    ParamInfo {
        name: param.declarator.0.clone(),
        mode,
        ty: param.ty.clone(),
    }
}

fn exception_unqualified(exception: &ScopedName) -> String {
    exception
        .name
        .last()
        .cloned()
        .unwrap_or_else(|| "Exception".to_string())
}
