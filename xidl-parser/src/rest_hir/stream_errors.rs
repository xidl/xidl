use super::{
    HttpException, HttpOperation, HttpResponseBodyShape, HttpStreamPayloadCodec,
    semantics::HttpStreamKind,
};
use crate::error::{ParseError, ParserResult};

impl HttpOperation {
    pub(super) fn validate_stream_exceptions(
        &self,
        exceptions: &[HttpException],
    ) -> ParserResult<()> {
        if self.meta.raises.is_empty() {
            return Ok(());
        }
        let name = &self.meta.name;
        if self.meta.upgrade_protocol.is_some()
            || self.meta.stream.kind == Some(HttpStreamKind::Bidi)
        {
            return Err(ParseError::Message(format!(
                "operation '{name}': raises(...) is not supported by WebSocket upgrade streams"
            )));
        }
        if self.meta.stream.kind == Some(HttpStreamKind::Server)
            && matches!(
                self.http.response.body.shape,
                HttpResponseBodyShape::Stream {
                    codec: HttpStreamPayloadCodec::Bytes,
                    ..
                }
            )
        {
            return Err(ParseError::Message(format!(
                "operation '{name}': raw byte response streams cannot carry terminal exceptions"
            )));
        }
        if self.meta.stream.kind.is_some() {
            for reference in &self.meta.raises {
                if let Some(exception) = exceptions.iter().find(|exception| {
                    exception.module_path == reference.module_path
                        && exception.ident == reference.ident
                }) && (!exception.headers.is_empty() || !exception.cookies.is_empty())
                {
                    return Err(ParseError::Message(format!(
                        "operation '{name}': stream exception '{}' cannot contain header or cookie members",
                        exception.ident
                    )));
                }
            }
        }
        Ok(())
    }
}
