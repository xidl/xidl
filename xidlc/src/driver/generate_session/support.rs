use crate::driver::generate_session::CodegenSession;
use crate::error::{IdlcError, IdlcResult};
use semver::{Version, VersionReq};

pub(super) fn verify_engine_version(session: &mut CodegenSession) -> IdlcResult<()> {
    let engine_req: String = session
        .get_engine_version()
        .map_err(|err| IdlcError::rpc(err.to_string()))?;
    let req = VersionReq::parse(&engine_req).map_err(|err| {
        IdlcError::rpc(format!(
            "invalid engine version requirement \"{engine_req}\": {err}"
        ))
    })?;
    let version = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|err| {
        IdlcError::rpc(format!(
            "invalid xidlc version \"{}\": {err}",
            env!("CARGO_PKG_VERSION")
        ))
    })?;
    if !req.matches(&version) {
        return Err(IdlcError::rpc(format!(
            "xidlc {version} is not compatible with engine requirement {engine_req}"
        )));
    }
    Ok(())
}
