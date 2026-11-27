use crate::driver::generate_session::CodegenSession;
use crate::driver::lang::Plugin;
use crate::error::{IdlcError, IdlcResult};
use std::collections::HashMap;
use xidl_parser::hir::ParserProperties;

/// Factory used when the pool has no checked-in session for a language.
pub(super) trait SessionSpawner: Send + Sync {
    /// Spawns a fresh session for `lang`.
    fn spawn(&mut self, lang: &str) -> IdlcResult<CodegenSession>;
}

impl<F> SessionSpawner for F
where
    F: FnMut(&str) -> IdlcResult<CodegenSession> + Send + Sync,
{
    fn spawn(&mut self, lang: &str) -> IdlcResult<CodegenSession> {
        self(lang)
    }
}

/// Cached generator sessions and the properties each language advertises.
pub(super) struct SessionPool {
    sessions: HashMap<String, CodegenSession>,
    properties: HashMap<String, ParserProperties>,
    spawn: Box<dyn SessionSpawner>,
}

impl SessionPool {
    /// Creates a pool that spawns built-in or external sessions on demand.
    pub(super) fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            properties: HashMap::new(),
            spawn: Box::new(CodegenSession::spawn),
        }
    }

    #[cfg(test)]
    pub(super) fn with_factory<F>(factory: F) -> Self
    where
        F: FnMut(&str) -> IdlcResult<CodegenSession> + Send + Sync + 'static,
    {
        Self {
            sessions: HashMap::new(),
            properties: HashMap::new(),
            spawn: Box::new(factory),
        }
    }

    /// Returns the advertised properties for `lang`, querying them at most once.
    pub(super) fn properties(&mut self, lang: &str) -> IdlcResult<ParserProperties> {
        let key = Plugin::from(lang).cache_key();
        if let Some(properties) = self.properties.get(&key) {
            return Ok(properties.clone());
        }
        let mut lease = self.checkout_key(&key)?;
        let properties = lease
            .session_mut()?
            .get_properties()
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        self.reclaim(lease);
        self.properties.insert(key, properties.clone());
        Ok(properties)
    }

    /// Takes the cached session for `lang`, spawning one when it is checked out.
    pub(super) fn checkout(&mut self, lang: &str) -> IdlcResult<SessionLease> {
        let key = Plugin::from(lang).cache_key();
        self.checkout_key(&key)
    }

    /// Returns a session to the pool, finishing a duplicate re-entrant session.
    pub(super) fn reclaim(&mut self, mut lease: SessionLease) {
        let Some(session) = lease.take() else {
            return;
        };
        if self.sessions.contains_key(&lease.lang) {
            session.finish();
        } else {
            self.sessions.insert(lease.lang.clone(), session);
        }
    }

    fn checkout_key(&mut self, key: &str) -> IdlcResult<SessionLease> {
        let session = match self.sessions.remove(key) {
            Some(session) => session,
            None => self.spawn.spawn(key)?,
        };
        Ok(SessionLease::new(key.to_owned(), session))
    }

    fn finish(&mut self) {
        for (_, session) in self.sessions.drain() {
            session.finish();
        }
    }
}

impl Drop for SessionPool {
    fn drop(&mut self) {
        self.finish();
    }
}

/// A session temporarily removed from [`SessionPool`].
pub(super) struct SessionLease {
    lang: String,
    session: Option<CodegenSession>,
}

impl SessionLease {
    fn new(lang: String, session: CodegenSession) -> Self {
        Self {
            lang,
            session: Some(session),
        }
    }

    /// Borrows the checked-out session.
    pub(super) fn session_mut(&mut self) -> IdlcResult<&mut CodegenSession> {
        self.session.as_mut().ok_or_else(|| {
            IdlcError::rpc("session pool: checked-out codegen session is unavailable")
        })
    }

    fn take(&mut self) -> Option<CodegenSession> {
        self.session.take()
    }
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            session.finish();
        }
    }
}
