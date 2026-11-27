//! Interface bindings assembled from the projected operation dispatchers.

use super::context::{Dispatcher, DispatcherVersion};
use crate::error::{IdlcError, IdlcResult};
use crate::generate::adapter::Operation;
use crate::generate::typescript::definition::{contexts::TsType, names::ts_ident};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct InterfaceBindings(BTreeMap<String, InterfaceOperations>);

struct InterfaceOperations {
    interface: String,
    table_path: Vec<String>,
    operations: BTreeMap<String, String>,
}

impl InterfaceBindings {
    pub(super) fn record(
        &mut self,
        operation: &Operation,
        interface: &str,
        key: &str,
    ) -> IdlcResult<()> {
        let group = self
            .0
            .entry(operation.interface.clone())
            .or_insert_with(|| {
                let interface = ts_ident(interface);
                let mut path = operation
                    .scope
                    .iter()
                    .map(|segment| ts_ident(segment))
                    .collect::<Vec<_>>();
                let adapter_name = if operation.scope.is_empty() {
                    interface.clone()
                } else {
                    format!("{}_{}", operation.scope.join("_"), interface)
                };
                path.push(format!("{interface}Operations"));
                InterfaceOperations {
                    interface: adapter_name,
                    table_path: path,
                    operations: BTreeMap::new(),
                }
            });
        let handler = ts_ident(&operation.name);
        if let Some(previous) = group.operations.insert(handler.clone(), key.to_owned())
            && previous != key
        {
            return Err(IdlcError::rpc(format!(
                "typescript adapters: handlers {previous} and {key} share {handler}"
            )));
        }
        Ok(())
    }

    pub(super) fn render(
        self,
        dispatchers: &BTreeMap<String, Dispatcher>,
        version_count: usize,
    ) -> IdlcResult<Vec<InterfaceDispatcher>> {
        self.0
            .into_values()
            .map(|group| group.render(dispatchers, version_count))
            .collect()
    }
}

impl InterfaceOperations {
    fn render(
        self,
        dispatchers: &BTreeMap<String, Dispatcher>,
        version_count: usize,
    ) -> IdlcResult<InterfaceDispatcher> {
        let mut entries = Vec::new();
        let mut readers = Vec::new();
        let mut mounted: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (handler, path) in self.operations {
            let dispatcher = dispatchers.get(&path).ok_or_else(|| {
                IdlcError::rpc(format!(
                    "typescript adapters: missing dispatcher for {path}"
                ))
            })?;
            for version in &dispatcher.versions {
                mounted
                    .entry(version.id.clone())
                    .or_default()
                    .push(handler.clone());
            }
            if let Some(current) = &dispatcher.current_type {
                readers.push(CurrentReader {
                    handler: handler.clone(),
                    result: current.clone(),
                    versions: dispatcher
                        .versions
                        .iter()
                        .filter(|version| version.current)
                        .cloned()
                        .collect(),
                });
            }
            entries.push(InterfaceDispatcherEntry::new(
                handler,
                path,
                dispatcher,
                version_count,
            ));
        }
        let interface = self.interface;
        Ok(InterfaceDispatcher {
            name: format!("{interface}Dispatchers"),
            source: format!("$xidl{interface}Source"),
            interface,
            table_path: self.table_path,
            entries,
            readers,
            mounts: mounted
                .into_iter()
                .map(|(id, handlers)| InterfaceMount { id, handlers })
                .collect(),
        })
    }
}

#[derive(Serialize)]
pub(super) struct InterfaceDispatcher {
    pub name: String,
    pub source: String,
    pub interface: String,
    pub table_path: Vec<String>,
    pub entries: Vec<InterfaceDispatcherEntry>,
    pub readers: Vec<CurrentReader>,
    pub mounts: Vec<InterfaceMount>,
}

/// The operations present in one projected server module.
#[derive(Serialize)]
pub(super) struct InterfaceMount {
    pub id: String,
    pub handlers: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct InterfaceDispatcherEntry {
    pub handler: String,
    pub prefix: String,
    pub service: HeadHandler,
    pub dispatch: Option<BindingDispatch>,
}

impl InterfaceDispatcherEntry {
    fn new(handler: String, path: String, dispatcher: &Dispatcher, version_count: usize) -> Self {
        let unavailable = dispatcher.versions.len() != version_count;
        let dispatch = (dispatcher.current || unavailable).then(|| BindingDispatch {
            cases: dispatcher
                .versions
                .iter()
                .map(|version| BindingCase {
                    id: version.id.clone(),
                    function: if version.current {
                        format!("$xidlDispatch{}From{}", dispatcher.prefix, version.pascal)
                    } else {
                        format!("dispatch{}From{}", dispatcher.prefix, version.pascal)
                    },
                    read_current: version.current,
                    supply_empty_stored: dispatcher.stored_type.is_some() && !version.stored,
                })
                .collect(),
            unavailable: unavailable.then_some(path),
        });
        Self {
            handler,
            prefix: dispatcher.prefix.clone(),
            service: HeadHandler {
                head_request: dispatcher.head_request.clone(),
                head_result: dispatcher.head_result.clone(),
                head_errors: dispatcher.head_errors.clone(),
                stored: dispatcher.stored,
                stored_type: dispatcher.stored_type.clone(),
            },
            dispatch,
        }
    }
}

#[derive(Serialize)]
pub(super) struct HeadHandler {
    pub head_request: TsType,
    pub head_result: TsType,
    pub head_errors: Vec<String>,
    pub stored: bool,
    pub stored_type: Option<String>,
}

#[derive(Serialize)]
pub(super) struct CurrentReader {
    pub handler: String,
    pub result: TsType,
    pub versions: Vec<DispatcherVersion>,
}

#[derive(Serialize)]
pub(super) struct BindingDispatch {
    pub cases: Vec<BindingCase>,
    pub unavailable: Option<String>,
}

#[derive(Serialize)]
pub(super) struct BindingCase {
    pub id: String,
    pub function: String,
    pub read_current: bool,
    pub supply_empty_stored: bool,
}
