//! Multi-error diagnostic collection for HTTP/REST projection rules.

use crate::diagnostic::{Diagnostic, DiagnosticBag, DiagnosticCode};
use crate::hir;

use super::attr::project_attribute_group;
use super::model::{HttpException, HttpUnion};
use super::project::project_operation;
use super::semantics::validate_http_annotations;
use super::validate::validate_unique_route_bindings_into;
use super::{HttpInterface, ProjectOptions};

const HTTP_ANNOTATION_NAMES: &[&str] = &[
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "head",
    "options",
    "http",
    "path",
    "query",
    "header",
    "cookie",
    "body",
    "upgrade",
    "cors",
    "http_basic",
    "http_bearer",
    "api_key",
    "oauth2",
    "no_security",
    "status",
    "stream_codec",
];

/// Collects HTTP/REST protocol diagnostics across `spec` without stopping at the first error.
pub fn collect_diagnostics(
    spec: &hir::Specification,
    options: &ProjectOptions,
    bag: &mut DiagnosticBag,
) {
    let mut collector = RestDiagnosticCollector::new(bag);
    collector.collect_exceptions(&spec.0, &[]);
    collector.http_unions = HttpUnion::analyze_into(&spec.0, false, collector.bag);
    collector.collect_interfaces(&spec.0, &[]);
    let RestDiagnosticCollector {
        bag,
        mut exceptions,
        mut interfaces,
        ..
    } = collector;
    if let Err(err) =
        super::types::TypeDeclarations::collect(&spec.0).normalize(&mut interfaces, &mut exceptions)
    {
        bag.push(Diagnostic::error(DiagnosticCode::ProtocolError, 0..0, err));
    }
    for interface in &interfaces {
        let iface_path = join_path(&interface.module_path, &interface.name);
        for operation in &interface.operations {
            if let Err(err) = operation.validate_stream_exceptions(&exceptions) {
                let op_path = format!("{iface_path}::{}", operation.meta.name);
                bag.push(
                    Diagnostic::error(DiagnosticCode::ProtocolError, 0..0, err.to_string())
                        .with_location(op_path),
                );
            }
        }
    }
    if let Some(parameter) = &options.version_parameter {
        if let Err(err) = parameter.validate() {
            bag.push(Diagnostic::error(DiagnosticCode::ProtocolError, 0..0, err));
        }
        if let Err(err) = parameter.validate_interfaces(&interfaces, &exceptions) {
            bag.push(Diagnostic::error(DiagnosticCode::ProtocolError, 0..0, err));
        }
    }
}

/// Returns `true` when any declaration in `spec` carries an HTTP-specific annotation.
pub(crate) fn has_http_annotations(spec: &hir::Specification) -> bool {
    defs_have_http_annotations(&spec.0)
}

struct RestDiagnosticCollector<'a> {
    bag: &'a mut DiagnosticBag,
    exceptions: Vec<HttpException>,
    http_unions: Vec<HttpUnion>,
    interfaces: Vec<HttpInterface>,
}

impl<'a> RestDiagnosticCollector<'a> {
    fn new(bag: &'a mut DiagnosticBag) -> Self {
        Self {
            bag,
            exceptions: Vec::new(),
            http_unions: Vec::new(),
            interfaces: Vec::new(),
        }
    }

    fn push_error(&mut self, location: Option<String>, message: String) {
        let mut diagnostic = Diagnostic::error(DiagnosticCode::ProtocolError, 0..0, message);
        if let Some(location) = location {
            diagnostic = diagnostic.with_location(location);
        }
        self.bag.push(diagnostic);
    }

    fn collect_exceptions(&mut self, definitions: &[hir::Definition], module_path: &[String]) {
        for def in definitions {
            match def {
                hir::Definition::ModuleDcl(module) => {
                    let mut next = module_path.to_vec();
                    next.push(module.ident.clone());
                    self.collect_exceptions(&module.definition, &next);
                }
                hir::Definition::ExceptDcl(except) => {
                    match HttpException::from_hir(except, module_path) {
                        Ok(exception) => self.exceptions.push(exception),
                        Err(err) => self.push_error(
                            Some(join_path(module_path, &except.ident)),
                            err.to_string(),
                        ),
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_interfaces(&mut self, definitions: &[hir::Definition], module_path: &[String]) {
        for def in definitions {
            match def {
                hir::Definition::ModuleDcl(module) => {
                    let mut next = module_path.to_vec();
                    next.push(module.ident.clone());
                    self.collect_interfaces(&module.definition, &next);
                }
                hir::Definition::InterfaceDcl(interface) => {
                    self.collect_interface(interface, module_path);
                }
                _ => {}
            }
        }
    }

    fn collect_interface(&mut self, interface: &hir::InterfaceDcl, module_path: &[String]) {
        let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
            return;
        };
        let iface_path = join_path(module_path, &def.header.ident);
        if let Err(err) = validate_http_annotations(
            &format!("interface '{}'", def.header.ident),
            &interface.annotations,
        ) {
            self.push_error(Some(iface_path.clone()), err);
        }

        let mut operations = Vec::new();
        if let Some(body) = &def.interface_body {
            for export in &body.0 {
                match export {
                    hir::Export::OpDcl(op) => {
                        match project_operation(
                            &def.header.ident,
                            module_path,
                            &interface.annotations,
                            &self.exceptions,
                            &self.http_unions,
                            op,
                        ) {
                            Ok(operation) => operations.push(operation),
                            Err(err) => self.push_error(
                                Some(format!("{iface_path}::{}", op.ident)),
                                err.to_string(),
                            ),
                        }
                    }
                    hir::Export::AttrDcl(attr) => {
                        match project_attribute_group(
                            &def.header.ident,
                            module_path,
                            &interface.annotations,
                            attr,
                        ) {
                            Ok(group) => operations.extend(group),
                            Err(err) => self.push_error(Some(iface_path.clone()), err.to_string()),
                        }
                    }
                    _ => {}
                }
            }
        }

        validate_unique_route_bindings_into(&def.header.ident, &operations, self.bag);
        self.interfaces.push(HttpInterface {
            name: def.header.ident.clone(),
            module_path: module_path.to_vec(),
            operations,
        });
    }
}

fn join_path(module_path: &[String], ident: &str) -> String {
    module_path
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(ident))
        .collect::<Vec<_>>()
        .join("::")
}

fn defs_have_http_annotations(definitions: &[hir::Definition]) -> bool {
    definitions.iter().any(|def| match def {
        hir::Definition::ModuleDcl(module) => {
            anns_have_http(&module.annotations) || defs_have_http_annotations(&module.definition)
        }
        hir::Definition::ExceptDcl(except) => anns_have_http(&except.annotations),
        hir::Definition::ConstrTypeDcl(constr) => constr_has_http(constr),
        hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(constr)) => constr_has_http(constr),
        hir::Definition::InterfaceDcl(interface) => {
            anns_have_http(&interface.annotations)
                || match &interface.decl {
                    hir::InterfaceDclInner::InterfaceDef(def) => {
                        def.interface_body.as_ref().is_some_and(|body| {
                            body.0.iter().any(|export| match export {
                                hir::Export::OpDcl(op) => {
                                    anns_have_http(&op.annotations)
                                        || op.parameter.as_ref().is_some_and(|params| {
                                            params
                                                .0
                                                .iter()
                                                .any(|param| anns_have_http(&param.annotations))
                                        })
                                }
                                hir::Export::AttrDcl(attr) => anns_have_http(&attr.annotations),
                                hir::Export::ExceptDcl(except) => {
                                    anns_have_http(&except.annotations)
                                }
                                hir::Export::TypeDcl(hir::TypeDcl::ConstrTypeDcl(constr)) => {
                                    constr_has_http(constr)
                                }
                                _ => false,
                            })
                        })
                    }
                    hir::InterfaceDclInner::InterfaceForwardDcl(_) => false,
                }
        }
        _ => false,
    })
}

fn constr_has_http(constr: &hir::ConstrTypeDcl) -> bool {
    match constr {
        hir::ConstrTypeDcl::UnionDef(union) => anns_have_http(&union.annotations),
        _ => false,
    }
}

fn anns_have_http(annotations: &[hir::Annotation]) -> bool {
    annotations.iter().any(|annotation| {
        hir::annotation_name(annotation).is_some_and(|name| {
            HTTP_ANNOTATION_NAMES
                .iter()
                .any(|http| name.eq_ignore_ascii_case(http))
        })
    })
}
