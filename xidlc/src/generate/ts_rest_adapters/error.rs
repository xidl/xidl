//! Adapter-local resolution of exception identifiers.

use xidl_parser::rest_hir::HttpException;

/// Mirror of the ordinary generator's exception ident rule: an exception whose
/// ident is unique among the contract's exceptions keeps its bare ident;
/// ambiguous idents qualify with their module path joined by `$`.
///
/// The adapter projects `raises(...)` into match arms without importing the
/// interface generator's model types.
pub(crate) fn error_ident(exception: &HttpException, exceptions: &[HttpException]) -> String {
    if exceptions
        .iter()
        .filter(|other| other.ident == exception.ident)
        .count()
        == 1
    {
        exception.ident.clone()
    } else {
        exception
            .module_path
            .iter()
            .chain(std::iter::once(&exception.ident))
            .cloned()
            .collect::<Vec<_>>()
            .join("$")
    }
}
